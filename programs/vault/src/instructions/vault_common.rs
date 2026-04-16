use anchor_lang::{
    prelude::*,
    solana_program::program::{get_return_data, invoke, invoke_signed},
};
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{
        self, approve_checked, burn, mint_to, ApproveChecked, Burn, Mint, MintTo, TokenAccount,
        TokenInterface, TransferChecked,
    },
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use crate::{
    error::VaultProgramError,
    extensions::{
        create_deposit_hook_ix, create_get_nav_ix, create_withdraw_hook_ix,
        get_deposit_hook_extra_account_metas_address,
        get_withdraw_hook_extra_account_metas_address, DepositHook, DepositHookInstruction,
        WithdrawHookInstruction,
    },
    state::{
        Rounding, Vault, VaultAction, VaultActionParams, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
    },
};

#[derive(Accounts)]
pub struct VaultCommon<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    // Not checking the mint authority is expected behaviour
    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
        seeds = [RESERVE_CONFIG_SEED, share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        token::authority = vault.fee_recipient,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = share_mint,
        token::token_program = share_token_program,
    )]
    pub user_shares_account: InterfaceAccount<'info, TokenAccount>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    pub extra_metas: Option<AccountInfo<'info>>,
    pub protocol: Option<AccountInfo<'info>>,
    pub hook_program: Option<AccountInfo<'info>>,
}

impl<'info> VaultCommon<'info> {
    /// Generic asset token transfer. Automatically uses vault PDA signing when
    /// authority is the vault, otherwise performs a plain user-signed transfer.
    pub fn transfer_asset_token(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from,
            mint: self.asset_mint.to_account_info(),
            to,
            authority: authority.clone(),
        };

        if authority.key() == self.vault.key() {
            let share_mint = self.share_mint.key();
            let seeds: &[&[&[u8]]] =
                &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
            let cpi_ctx = CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                cpi_accounts,
                seeds,
            );
            token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
        } else {
            let cpi_ctx = CpiContext::new(self.asset_token_program.to_account_info(), cpi_accounts);
            token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
        }
    }

    /// Returns the Token-2022 transfer fee for the given amount, or 0 for standard SPL tokens.
    pub fn get_transfer_fees(&mut self, amount: u64) -> Result<u64> {
        if self.asset_mint.to_account_info().owner == &spl_token::id() {
            return Ok(0);
        }
        let binding = self.asset_mint.to_account_info();
        let mint_data = binding.data.borrow();
        let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
        let epoch = Clock::get()?.epoch;
        let fee = match mint
            .get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>()
        {
            Ok(cfg) => cfg
                .calculate_epoch_fee(epoch, amount)
                .ok_or(VaultProgramError::ArithmeticError)?,
            Err(_) => 0,
        };
        Ok(fee)
    }

    /// Grants a delegate account permission to transfer up to `amount` tokens from the reserve,
    /// signed by the vault PDA (which owns the reserve).
    pub fn delegate_reserve(&mut self, delegate: AccountInfo<'info>, amount: u64) -> Result<()> {
        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let approve_cpi_accounts = ApproveChecked {
            to: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            delegate,
            authority: self.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            self.asset_token_program.to_account_info(),
            approve_cpi_accounts,
            seeds,
        );
        approve_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    /// Share functions

    /// Mints the calculated share amount to the user's share token account, signed by the vault
    /// PDA.
    pub fn mint_shares_to_user(&mut self, amount: u64) -> Result<()> {
        let share_mint = self.share_mint.key();
        let mint_to_cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: self.user_shares_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        let mint_cpi_ctx = CpiContext::new_with_signer(
            self.share_token_program.to_account_info(),
            mint_to_cpi_accounts,
            seeds,
        );
        mint_to(mint_cpi_ctx, amount)
    }

    /// Burns `shares_amount` from user shares token account
    pub fn burn_shares(&mut self, shares_amount: u64) -> Result<()> {
        let cpi_accounts = Burn {
            mint: self.share_mint.to_account_info(),
            from: self.user_shares_account.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.share_token_program.to_account_info(), cpi_accounts);

        burn(cpi_ctx, shares_amount)
    }

    /// Transfers assets from the user to the reserve and reloads the reserve account.
    pub fn deposit_to_reserve(&mut self, amount: u64) -> Result<()> {
        self.transfer_asset_token(
            self.user_assets_account.to_account_info(),
            self.reserve.to_account_info(),
            self.user.to_account_info(),
            amount,
        )?;
        self.reserve.reload()
    }

    /// Transfers the deposit fee to the fee recipient and mints shares to the user.
    pub fn collect_fee_and_mint_shares(&mut self, fee: u64, shares: u64) -> Result<()> {
        self.transfer_asset_token(
            self.user_assets_account.to_account_info(),
            self.fee_recipient.to_account_info(),
            self.user.to_account_info(),
            fee,
        )?;
        self.mint_shares_to_user(shares)
    }

    /// Hook Program

    /// Invokes the hook program's `get_nav` instruction via CPI to retrieve the current
    /// Net Asset Value of the vault.
    ///
    /// The first entry in `remaining_accounts` must be the `associated_protocols_info` PDA,
    /// followed by each protocol's `AssociatedProtocol` PDA and its token account.
    pub fn get_nav(
        &self,
        hook_program: Pubkey,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<u64> {
        let associated_protocol = &*remaining_accounts
            .first()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let instruction = create_get_nav_ix(
            &hook_program,
            &self.share_mint.key(),
            associated_protocol.key,
        );

        let cpi_account_infos = vec![
            self.share_mint.to_account_info(),
            associated_protocol.to_account_info(),
        ];

        invoke(&instruction, &cpi_account_infos)?;

        let (return_program_id, return_data) =
            get_return_data().ok_or(VaultProgramError::StaleVaultNav)?;

        require_keys_eq!(
            return_program_id,
            hook_program,
            VaultProgramError::InvalidReturnedData
        );
        require!(
            return_data.len() >= 8,
            VaultProgramError::InvalidReturnedData
        );

        let nav = u64::from_le_bytes(
            return_data[0..8]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        Ok(nav)
    }

    /// Invokes the deposit hook program, allowing it to execute custom logic on deposit.
    ///
    /// The deposit hook is an external program registered on the vault that can
    /// intercept each deposit to implement custom behaviour.
    /// Account resolution follows the `ExtraAccountMetaList` pattern.
    pub fn execute_deposit_hook(
        &mut self,
        hook_program: Pubkey,
        remaining_accounts: &[AccountInfo<'info>],
        deposit_amount: u64,
    ) -> Result<u64> {
        let extra_metas = &self
            .extra_metas
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let protocol = &self
            .protocol
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let share_mint = self.share_mint.key();

        let mut instruction = create_deposit_hook_ix(
            &hook_program,
            &self.vault.key(),
            &self.share_mint.key(),
            &extra_metas.key(),
            &protocol.key(),
            &self.system_program.key(),
            deposit_amount,
        );

        let validation_pubkey =
            get_deposit_hook_extra_account_metas_address(&self.share_mint.key(), &hook_program);

        let mut cpi_account_infos = vec![
            self.vault.to_account_info(),
            self.share_mint.to_account_info(),
            extra_metas.to_account_info(),
            protocol.to_account_info(),
            self.system_program.to_account_info(),
        ];

        if extra_metas.key() == validation_pubkey {
            // Append the validation account itself, then let the SPL TLV library resolve and
            // append any additional accounts declared in the ExtraAccountMetaList.
            instruction
                .accounts
                .push(AccountMeta::new_readonly(validation_pubkey, false));
            let validation_info = extra_metas.to_account_info();
            cpi_account_infos.push(validation_info.clone());
            ExtraAccountMetaList::add_to_cpi_instruction::<DepositHookInstruction>(
                &mut instruction,
                &mut cpi_account_infos,
                &validation_info.try_borrow_data()?,
                remaining_accounts,
            )?;
        } else {
            return Err(VaultProgramError::InvalidAccountData.into());
        }

        // The vault PDA signs so the hook program can authenticate this call.
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        // Forward remaining accounts into the hook instruction so they are
        // visible as ctx.remaining_accounts inside the hook program.
        for account in remaining_accounts.iter() {
            instruction.accounts.push(AccountMeta {
                pubkey: account.key(),
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            });
        }
        cpi_account_infos.extend_from_slice(remaining_accounts);

        invoke_signed(&instruction, &cpi_account_infos, seeds)?;
        let (return_program_id, return_data) =
            get_return_data().ok_or(VaultProgramError::StaleVaultNav)?;

        // Ensure the return data originates from the expected hook program and not a spoofed one.
        require_keys_eq!(
            return_program_id,
            hook_program.key(),
            VaultProgramError::InvalidReturnedData
        );
        require!(
            return_data.len() >= 8,
            VaultProgramError::InvalidReturnedData
        );

        let nav = u64::from_le_bytes(
            return_data[0..8]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        Ok(nav)
    }

    /// Invokes the withdraw hook program, allowing it to execute custom logic on withdrawal.
    pub fn execute_withdraw_hook(
        &mut self,
        hook_program: Pubkey,
        remaining_accounts: &[AccountInfo<'info>],
        withdraw_amount: u64,
    ) -> Result<u64> {
        let extra_metas = &self
            .extra_metas
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let protocol = &self
            .protocol
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let share_mint = self.share_mint.key();

        let mut instruction = create_withdraw_hook_ix(
            &hook_program,
            &self.vault.key(),
            &self.share_mint.key(),
            &extra_metas.key(),
            &protocol.key(),
            &self.system_program.key(),
            withdraw_amount,
        );

        let validation_pubkey =
            get_withdraw_hook_extra_account_metas_address(&self.share_mint.key(), &hook_program);

        let mut cpi_account_infos = vec![
            self.vault.to_account_info(),
            self.share_mint.to_account_info(),
            extra_metas.to_account_info(),
            protocol.to_account_info(),
            self.system_program.to_account_info(),
        ];

        if extra_metas.key() == validation_pubkey {
            instruction
                .accounts
                .push(AccountMeta::new_readonly(validation_pubkey, false));
            let validation_info = extra_metas.to_account_info();
            cpi_account_infos.push(validation_info.clone());
            ExtraAccountMetaList::add_to_cpi_instruction::<WithdrawHookInstruction>(
                &mut instruction,
                &mut cpi_account_infos,
                &validation_info.try_borrow_data()?,
                remaining_accounts,
            )?;
        } else {
            return Err(VaultProgramError::InvalidAccountData.into());
        }

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        // Forward remaining accounts into the hook instruction so they are
        // visible as ctx.remaining_accounts inside the hook program.
        for account in remaining_accounts.iter() {
            instruction.accounts.push(AccountMeta {
                pubkey: account.key(),
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            });
        }
        cpi_account_infos.extend_from_slice(remaining_accounts);

        invoke_signed(&instruction, &cpi_account_infos, seeds)?;
        let (return_program_id, return_data) =
            get_return_data().ok_or(VaultProgramError::StaleVaultNav)?;

        // Ensure the return data originates from the expected hook program and not a spoofed one.
        require_keys_eq!(
            return_program_id,
            hook_program.key(),
            VaultProgramError::InvalidReturnedData
        );
        require!(
            return_data.len() >= 8,
            VaultProgramError::InvalidReturnedData
        );

        let nav = u64::from_le_bytes(
            return_data[0..8]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        Ok(nav)
    }

    /// Burns shares, transfers the withdrawal fee from the reserve to the fee
    /// recipient, and sends the remaining assets from the reserve to the user.
    pub fn burn_and_distribute(
        &mut self,
        shares_to_burn: u64,
        fee: u64,
        user_assets_out: u64,
    ) -> Result<()> {
        self.burn_shares(shares_to_burn)?;

        if fee > 0 {
            self.transfer_asset_token(
                self.reserve.to_account_info(),
                self.fee_recipient.to_account_info(),
                self.vault.to_account_info(),
                fee,
            )?;
        }
        self.transfer_asset_token(
            self.reserve.to_account_info(),
            self.user_assets_account.to_account_info(),
            self.vault.to_account_info(),
            user_assets_out,
        )
    }

    pub fn get_program_hook_pubkey(
        &mut self,
        deposit_hook_program: Option<DepositHook>,
    ) -> Result<Pubkey> {
        let deposit_hook =
            deposit_hook_program.ok_or(VaultProgramError::HookExtensionNotInitialized)?;
        let hook_program_pubkey = deposit_hook.hook_program_id;
        let hook_program_account = self
            .hook_program
            .as_ref()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        require!(
            hook_program_account.key().eq(&hook_program_pubkey),
            VaultProgramError::HookExtensionNotInitialized
        );
        Ok(hook_program_pubkey)
    }
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    kind: VaultAction,
    args: VaultActionParams,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    match kind {
        // User specifies exact asset amount in, receives shares.
        // threshold_amount = min shares out (slippage).
        VaultAction::Deposit => {
            let assets = args.amount;
            let min_shares = args.threshold_amount;

            let fee = ctx.accounts.vault.get_deposit_fee(assets)?;
            let reserve_amount_before = ctx.accounts.reserve.amount;
            let remaining_amount = assets
                .checked_sub(fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            ctx.accounts.deposit_to_reserve(remaining_amount)?;

            let deposit_hook_program = ctx.accounts.vault.deposit_hook_type();
            let updated_reserve_amount = ctx.accounts.reserve.amount;
            let actual_transferred_amount = updated_reserve_amount
                .checked_sub(reserve_amount_before)
                .ok_or(VaultProgramError::ArithmeticError)?;

            let shares = if deposit_hook_program.is_some() {
                let hook_program_pubkey =
                    ctx.accounts.get_program_hook_pubkey(deposit_hook_program)?;
                let remaining = ctx.remaining_accounts;
                ctx.accounts.delegate_reserve(
                    ctx.accounts
                        .hook_program
                        .as_ref()
                        .ok_or(VaultProgramError::OptionalAccountIsEmpty)?
                        .clone(),
                    actual_transferred_amount,
                )?;
                let nav = ctx.accounts.execute_deposit_hook(
                    hook_program_pubkey,
                    remaining,
                    actual_transferred_amount,
                )?;

                // Convert NAV (price per share = assets * 10^decimals / supply)
                // back to total assets for the cap check.
                let total_assets = if ctx.accounts.share_mint.supply == 0 {
                    // First deposit: no shares exist yet so NAV is 0.
                    // Total assets equals the deposited amount.
                    actual_transferred_amount
                } else {
                    let precision = 10u128.pow(ctx.accounts.share_mint.decimals as u32);
                    let total = u128::from(nav)
                        .checked_mul(u128::from(ctx.accounts.share_mint.supply))
                        .ok_or(VaultProgramError::ArithmeticError)?
                        .checked_div(precision)
                        .ok_or(VaultProgramError::ArithmeticError)?;
                    u64::try_from(total).map_err(|_| VaultProgramError::ArithmeticError)?
                };

                require!(
                    total_assets <= ctx.accounts.vault.vault_asset_cap,
                    VaultProgramError::MaxVaultAssetCapExceeded
                );

                // Derive shares from NAV (mirrors the withdraw-hook approach).
                if ctx.accounts.share_mint.supply == 0 {
                    // First deposit: use initial_price as the exchange rate.
                    let shares = u128::from(ctx.accounts.vault.initial_price)
                        .checked_mul(u128::from(actual_transferred_amount))
                        .ok_or(VaultProgramError::ArithmeticError)?;
                    u64::try_from(shares).map_err(|_| VaultProgramError::ArithmeticError)?
                } else {
                    require!(nav > 0, VaultProgramError::StaleVaultNav);
                    let precision = 10u128.pow(ctx.accounts.share_mint.decimals as u32);
                    u64::try_from(
                        u128::from(actual_transferred_amount)
                            .checked_mul(precision)
                            .ok_or(VaultProgramError::ArithmeticError)?
                            .checked_div(u128::from(nav))
                            .ok_or(VaultProgramError::ArithmeticError)?,
                    )
                    .map_err(|_| VaultProgramError::ArithmeticError)?
                }
            } else {
                require!(
                    updated_reserve_amount <= ctx.accounts.vault.vault_asset_cap,
                    VaultProgramError::MaxVaultAssetCapExceeded
                );

                ctx.accounts.vault.get_shares_from_assets(
                    reserve_amount_before,
                    ctx.accounts.share_mint.supply,
                    actual_transferred_amount,
                    Rounding::Down,
                )?
            };

            if shares == 0 {
                return Err(VaultProgramError::InsufficientDepositAmount.into());
            }
            if shares < min_shares {
                return Err(VaultProgramError::SlippageExceeded.into());
            }

            ctx.accounts.collect_fee_and_mint_shares(fee, shares)?;
        }

        // User specifies exact share amount to mint, pays required assets.
        // threshold_amount = max assets in (slippage).
        VaultAction::Mint => {
            require!(
                ctx.accounts.vault.deposit_hook_type().is_none(),
                VaultProgramError::HookExtensionActive
            );

            let shares = args.amount;
            let max_assets = args.threshold_amount;

            let assets = ctx.accounts.vault.get_assets_from_shares(
                ctx.accounts.reserve.amount,
                ctx.accounts.share_mint.supply,
                shares,
                Rounding::Up,
            )?;

            if assets > max_assets {
                return Err(VaultProgramError::SlippageExceeded.into());
            }

            let transfer_fee = ctx.accounts.get_transfer_fees(assets)?;
            let assets_plus_transfer_fee = assets
                .checked_add(transfer_fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            ctx.accounts.deposit_to_reserve(assets_plus_transfer_fee)?;

            require!(
                ctx.accounts.reserve.amount <= ctx.accounts.vault.vault_asset_cap,
                VaultProgramError::MaxVaultAssetCapExceeded
            );

            let fee = ctx.accounts.vault.get_deposit_fee_when_minting(assets)?;
            ctx.accounts.collect_fee_and_mint_shares(fee, shares)?;
        }

        // User specifies exact asset amount out, burns required shares.
        // threshold_amount = max shares burned (slippage).
        VaultAction::Withdraw => {
            let assets = args.amount;
            let max_shares = args.threshold_amount;

            let fee = ctx.accounts.vault.get_withdraw_fee(assets)?;
            let amount_with_fee = assets
                .checked_add(fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            let withdraw_hook_program = ctx.accounts.vault.withdraw_hook_type();
            let shares_to_burn = if let Some(withdraw_hook) = withdraw_hook_program {
                let hook_program_pubkey = withdraw_hook.hook_program_id;
                let hook_program_account = ctx
                    .accounts
                    .hook_program
                    .as_ref()
                    .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
                require!(
                    hook_program_account.key().eq(&hook_program_pubkey),
                    VaultProgramError::HookExtensionNotInitialized
                );
                let remaining = ctx.remaining_accounts;
                let nav =
                    ctx.accounts
                        .execute_withdraw_hook(hook_program_pubkey, remaining, assets)?;

                require!(nav > 0, VaultProgramError::StaleVaultNav);
                let precision = 10u128.pow(ctx.accounts.share_mint.decimals as u32);
                u64::try_from(
                    u128::from(amount_with_fee)
                        .checked_mul(precision)
                        .ok_or(VaultProgramError::ArithmeticError)?
                        .div_ceil(u128::from(nav)),
                )
                .map_err(|_| VaultProgramError::ArithmeticError)?
            } else {
                ctx.accounts.vault.get_shares_from_assets(
                    ctx.accounts.reserve.amount,
                    ctx.accounts.share_mint.supply,
                    amount_with_fee,
                    Rounding::Up,
                )?
            };

            if shares_to_burn == 0 {
                return Err(VaultProgramError::InsufficientWithdrawAmount.into());
            }
            if shares_to_burn > max_shares {
                return Err(VaultProgramError::SlippageExceeded.into());
            }

            ctx.accounts
                .burn_and_distribute(shares_to_burn, fee, assets)?;
        }

        // User specifies exact share amount to burn, receives assets.
        // threshold_amount = min assets out (slippage).
        VaultAction::Redeem => {
            require!(
                ctx.accounts.vault.withdraw_hook_type().is_none(),
                VaultProgramError::HookExtensionActive
            );

            let shares = args.amount;
            let min_assets = args.threshold_amount;

            require!(shares > 0, VaultProgramError::InsufficientRedeemAmount);
            require!(
                ctx.accounts.share_mint.supply > 0,
                VaultProgramError::InvalidState
            );

            let total_assets_out = ctx.accounts.vault.get_assets_from_shares(
                ctx.accounts.reserve.amount,
                ctx.accounts.share_mint.supply,
                shares,
                Rounding::Down,
            )?;

            if total_assets_out == 0 {
                return Err(VaultProgramError::InsufficientRedeemAmount.into());
            }

            let fee = ctx
                .accounts
                .vault
                .get_withdraw_fee_when_redeeming(total_assets_out)?;
            let user_assets_out = total_assets_out
                .checked_sub(fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            if user_assets_out < min_assets {
                return Err(VaultProgramError::SlippageExceeded.into());
            }

            ctx.accounts
                .burn_and_distribute(shares, fee, user_assets_out)?;
        }
    }

    Ok(())
}

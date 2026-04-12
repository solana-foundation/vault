use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program::{get_return_data, invoke, invoke_signed},
    },
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
        Rounding, SwapKind, SwapParams, Vault, MAX_BPS, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
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

    pub extra_metas: Option<AccountInfo<'info>>,
    pub protocol: Option<AccountInfo<'info>>,
    pub hook_program: Option<AccountInfo<'info>>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
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
    ) -> Result<(u64, u64)> {
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
            return_data.len() >= 16,
            VaultProgramError::InvalidReturnedData
        );

        let shares = u64::from_le_bytes(
            return_data[0..8]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        let total_nav = u64::from_le_bytes(
            return_data[8..16]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        Ok((shares, total_nav))
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
    kind: SwapKind,
    args: SwapParams,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    match kind {
        // User specifies exact asset amount in, receives shares.
        // threshold_amount = min shares out (slippage).
        SwapKind::Deposit => {
            let assets = args.amount;
            let min_shares = args.threshold_amount;

            let fee = ctx.accounts.vault.get_deposit_fee(assets)?;
            let reserve_amount_before = ctx.accounts.reserve.amount;
            let remaining_amount = assets
                .checked_sub(fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            ctx.accounts.transfer_asset_token(
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.reserve.to_account_info(),
                ctx.accounts.user.to_account_info(),
                remaining_amount,
            )?;
            ctx.accounts.reserve.reload()?;

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
                let (shares, total_nav) = ctx.accounts.execute_deposit_hook(
                    hook_program_pubkey,
                    remaining,
                    actual_transferred_amount,
                )?;

                // When deposit hook is active, assets are deployed to protocols
                // so we check total NAV (reserve + protocols) against the cap.
                require!(
                    total_nav <= ctx.accounts.vault.vault_asset_cap,
                    VaultProgramError::MaxVaultAssetCapExceeded
                );

                shares
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

            ctx.accounts.transfer_asset_token(
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.user.to_account_info(),
                fee,
            )?;
            ctx.accounts.mint_shares_to_user(shares)?;
        }

        // User specifies exact share amount to mint, pays required assets.
        // threshold_amount = max assets in (slippage).
        SwapKind::Mint => {
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

            ctx.accounts.transfer_asset_token(
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.reserve.to_account_info(),
                ctx.accounts.user.to_account_info(),
                assets_plus_transfer_fee,
            )?;
            ctx.accounts.reserve.reload()?;

            require!(
                ctx.accounts.reserve.amount <= ctx.accounts.vault.vault_asset_cap,
                VaultProgramError::MaxVaultAssetCapExceeded
            );

            let fee = ctx.accounts.vault.get_deposit_fee_when_minting(assets)?;
            ctx.accounts.transfer_asset_token(
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.user.to_account_info(),
                fee,
            )?;
            ctx.accounts.mint_shares_to_user(shares)?;
        }

        // User specifies exact asset amount out, burns required shares.
        // threshold_amount = max shares burned (slippage).
        SwapKind::Withdraw => {
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
                ctx.accounts
                    .execute_withdraw_hook(hook_program_pubkey, remaining, assets)?
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

            ctx.accounts.burn_shares(shares_to_burn)?;

            if fee > 0 {
                ctx.accounts.transfer_asset_token(
                    ctx.accounts.reserve.to_account_info(),
                    ctx.accounts.fee_recipient.to_account_info(),
                    ctx.accounts.vault.to_account_info(),
                    fee,
                )?;
            }
            ctx.accounts.transfer_asset_token(
                ctx.accounts.reserve.to_account_info(),
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                assets,
            )?;
        }

        // User specifies exact share amount to burn, receives assets.
        // threshold_amount = min assets out (slippage).
        SwapKind::Redeem => {
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

            ctx.accounts.burn_shares(shares)?;

            if fee > 0 {
                ctx.accounts.transfer_asset_token(
                    ctx.accounts.reserve.to_account_info(),
                    ctx.accounts.fee_recipient.to_account_info(),
                    ctx.accounts.vault.to_account_info(),
                    fee,
                )?;
            }
            ctx.accounts.transfer_asset_token(
                ctx.accounts.reserve.to_account_info(),
                ctx.accounts.user_assets_account.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                user_assets_out,
            )?;
        }
    }

    Ok(())
}

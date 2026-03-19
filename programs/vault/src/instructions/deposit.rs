use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program::{get_return_data, invoke_signed},
    },
};
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{self, mint_to, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked},
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use crate::{
    error::VaultProgramError,
    extensions::{
        create_deposit_hook_ix, get_deposit_hook_extra_account_metas_address,
        DepositHookInstruction,
    },
    state::{
        Rounding, VaultConfig, GET_NAV_DISCRIMINATOR, MAX_BPS, RESERVE_CONFIG_SEED,
        VAULT_CONFIG_SEED,
    },
};

#[derive(Accounts)]
pub struct DepositAndMint<'info> {
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
    pub vault: Account<'info, VaultConfig>,

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
    pub nav_return_data: Option<AccountInfo<'info>>,
    pub hook_program: Option<AccountInfo<'info>>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub instructions: Option<AccountInfo<'info>>,
}

impl<'info> DepositAndMint<'info> {
    /// Transfers the deposit fee from the user's asset account to the fee recipient.
    pub fn transfer_asset_token_fee_to_fee_recipient(&mut self, fee: u64) -> Result<()> {
        let fee_recipient_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.fee_recipient.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            self.asset_token_program.to_account_info(),
            fee_recipient_transfer_cpi_accounts,
        );

        token_interface::transfer_checked(cpi_ctx, fee, self.asset_mint.decimals)
    }

    /// Transfers the deposit amount from the user's asset account into the vault reserve.
    pub fn transfer_asset_token_to_vault(&mut self, amount: u64) -> Result<()> {
        let vault_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.reserve.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.asset_token_program.to_account_info(),
            vault_transfer_cpi_accounts,
        );
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

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

    /// Returns the Token-2022 transfer fee for the given amount, or 0 for standard SPL tokens.
    pub fn get_transfer_fees(&mut self, amount: u64) -> Result<u64> {
        if self.asset_mint.to_account_info().owner == &spl_token::id() {
            return Ok(0);
        }
        let binding = self.asset_mint.to_account_info();
        let mint_data = binding.data.borrow();
        let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
        let transfer_fee_config =
            mint.get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>()?;
        let transfer_fee: u16 = transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points
            .into();
        Ok(amount
            .checked_mul(transfer_fee.into())
            .ok_or(VaultProgramError::ArithmeticError)?
            .div_ceil(MAX_BPS.into()))
    }

    /// Queries the hook program for the current Net Asset Value (NAV) of the vault.
    ///
    /// The NAV is needed to correctly
    /// calculate the share-to-asset exchange rate when the vault has an active deposit hook.
    pub fn get_nav_value(
        &self,
        hook_program: Pubkey,
        hook_program_account_info: AccountInfo<'info>,
    ) -> Result<u64> {
        let nav_account = self
            .nav_return_data
            .as_ref()
            .ok_or(VaultProgramError::StaleVaultNav)?;

        let ix_sysvar = self
            .instructions
            .as_ref()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;

        // Build the `get_nav` CPI instruction targeting the hook program.
        // The hook program reads the vault state and writes the NAV into return data.
        let get_nav_ix = Instruction {
            program_id: hook_program,
            accounts: vec![
                AccountMeta::new_readonly(self.vault.key(), false),
                AccountMeta::new_readonly(*nav_account.key, false),
                AccountMeta::new_readonly(
                    anchor_lang::solana_program::sysvar::instructions::ID,
                    false,
                ),
            ],
            data: GET_NAV_DISCRIMINATOR.to_vec(),
        };

        invoke_signed(
            &get_nav_ix,
            &[
                hook_program_account_info,
                self.vault.to_account_info(),
                nav_account.clone(),
                ix_sysvar.clone(),
            ],
            &[],
        )?;

        // Retrieve and validate the return data written by the hook program.
        let (return_program_id, return_data) =
            get_return_data().ok_or(VaultProgramError::StaleVaultNav)?;
        // Ensure the return data originates from the expected hook program and not a spoofed one.
        require_keys_eq!(
            return_program_id,
            hook_program.key(),
            VaultProgramError::InvalidReturnedData
        );
        require!(
            return_data.len() >= 24,
            VaultProgramError::InvalidReturnedData
        );

        // Reject stale NAV values to prevent deposits from using an outdated exchange rate.
        let update_timestamp = i64::from_le_bytes(
            return_data[16..24]
                .try_into()
                .map_err(|_| VaultProgramError::InvalidReturnedData)?,
        );

        let current_time = Clock::get()?.unix_timestamp;
        let nav_age = current_time
            .checked_sub(update_timestamp)
            .ok_or(VaultProgramError::ArithmeticError)?;
        require!(nav_age <= 60, VaultProgramError::StaleVaultNav);
        let nav = u64::from_le_bytes(
            return_data[8..16]
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
    ) -> Result<()> {
        let extra_metas = &self
            .extra_metas
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let protocol = &self
            .protocol
            .clone()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        let share_mint = self.share_mint.key();

        // Build the base deposit-hook instruction with the standard set of accounts.
        let mut instruction = create_deposit_hook_ix(
            &hook_program,
            &self.vault.key(),
            &self.share_mint.key(),
            &extra_metas.key(),
            &protocol.key(),
            &self.system_program.key(),
        );

        // Derive the expected address of the extra-account-metas validation PDA to guard
        // against a caller passing a malicious account in place of the real one.
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

        cpi_account_infos.extend_from_slice(remaining_accounts);

        invoke_signed(&instruction, &cpi_account_infos, seeds)?;
        Ok(())
    }
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, DepositAndMint<'info>>,
    assets: u64,
    min_shares: u64,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    let fee = ctx.accounts.vault.get_deposit_fee(assets)?;
    // current vault amount
    let reserve_amount_before = ctx.accounts.reserve.amount;
    // transfer assets in case there are transfer fees (Token2022)
    let remaining_amount = assets
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    ctx.accounts
        .transfer_asset_token_to_vault(remaining_amount)?;
    ctx.accounts.reserve.reload()?;

    let deposit_hook_program = ctx.accounts.vault.deposit_hook_type();
    let mut reserve_balance = reserve_amount_before;
    if deposit_hook_program.is_some() {
        let hook_program_pubkey =
            deposit_hook_program.ok_or(VaultProgramError::HookExtensionNotInitialized)?;
        let hook_program_account = ctx
            .accounts
            .hook_program
            .as_ref()
            .ok_or(VaultProgramError::OptionalAccountIsEmpty)?;
        require!(
            hook_program_account.key().eq(&hook_program_pubkey),
            VaultProgramError::HookExtensionNotInitialized
        );
        let nav = ctx
            .accounts
            .get_nav_value(hook_program_pubkey, hook_program_account.clone())?;
        let remaining = ctx.remaining_accounts;
        // Delegate
        ctx.accounts
            .execute_deposit_hook(hook_program_pubkey, remaining)?;
        // Remove delegation
        reserve_balance = reserve_balance
            .checked_add(nav)
            .ok_or(VaultProgramError::ArithmeticError)?;
    }

    let updated_reserve_amount = ctx.accounts.reserve.amount;

    let actual_transferred_amount = updated_reserve_amount
        .checked_sub(reserve_amount_before)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        updated_reserve_amount <= ctx.accounts.vault.vault_asset_cap,
        VaultProgramError::MaxVaultAssetCapExceeded
    );

    let shares = ctx.accounts.vault.get_shares_from_assets(
        reserve_balance,
        ctx.accounts.share_mint.supply,
        actual_transferred_amount,
        Rounding::Down,
    )?;

    if shares == 0 {
        return Err(VaultProgramError::InsufficientDepositAmount.into());
    }

    if shares < min_shares {
        return Err(VaultProgramError::SlippageExceeded.into());
    }

    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

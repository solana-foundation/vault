use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program::{get_return_data, invoke_signed},
    },
};
use anchor_spl::token_interface::{
    self, burn, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use crate::{
    error::VaultProgramError,
    extensions::{
        create_withdraw_hook_ix, get_withdraw_hook_extra_account_metas_address,
        WithdrawHookInstruction,
    },
    state::{Rounding, VaultConfig, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// `User` that is withdrawing assets from `Vault`
    #[account(mut)]
    pub user: Signer<'info>,

    /// Mint of the underlying asset
    pub asset_mint: InterfaceAccount<'info, Mint>,

    /// Share mint
    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    /// Vault reserve token account holding underlying assets
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
        seeds = [RESERVE_CONFIG_SEED, share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    /// Vault configuration account (PDA)
    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    /// Fee recipient token account
    #[account(
        mut,
        token::authority = vault.fee_recipient,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    /// User's asset token account
    #[account(
        mut,
        token::authority = user,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    /// User's share token account
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

    pub share_token_program: Interface<'info, TokenInterface>,
    pub asset_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_assets_to_fee_recipient(&mut self, fee: u64) -> Result<()> {
        let share_mint = self.share_mint.key();

        let cpi_accounts = TransferChecked {
            from: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.fee_recipient.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.asset_token_program.to_account_info(),
            cpi_accounts,
            seeds,
        );

        token_interface::transfer_checked(cpi_ctx, fee, self.asset_mint.decimals)
    }

    /// Transfers `asset_amount` tokens to the user token account
    pub fn transfer_assets_to_user(&mut self, asset_amount: u64) -> Result<()> {
        let share_mint = self.share_mint.key();

        let cpi_accounts = TransferChecked {
            from: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.user_assets_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.asset_token_program.to_account_info(),
            cpi_accounts,
            seeds,
        );

        token_interface::transfer_checked(cpi_ctx, asset_amount, self.asset_mint.decimals)
    }

    /// Invokes the withdraw hook program, allowing it to execute custom logic on withdrawal.
    pub fn execute_withdraw_hook(
        &mut self,
        hook_program: Pubkey,
        remaining_accounts: &[AccountInfo<'info>],
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
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>,
    assets: u64,
    max_shares: u64,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    // assets is NET to receiver/user.
    let amount_assets_out = assets;

    // fee computed on the net amount
    let fee = ctx.accounts.vault.get_withdraw_fee(amount_assets_out)?;

    // total assets leaving the vault reserve including fees
    let amount_with_fee = amount_assets_out
        .checked_add(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    // If a withdraw hook is registered, use the NAV reported by the hook program
    // as the reserve balance (assets may be deployed externally).
    let withdraw_hook_program = ctx.accounts.vault.withdraw_hook_type();
    let reserve_balance = if let Some(withdraw_hook) = withdraw_hook_program {
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
        let hook_program_pubkey = withdraw_hook.hook_program_id;
        let remaining = ctx.remaining_accounts;
        ctx.accounts
            .execute_withdraw_hook(hook_program_pubkey, remaining)?
    } else {
        ctx.accounts.reserve.amount
    };

    let shares_to_burn = ctx.accounts.vault.get_shares_from_assets(
        reserve_balance,
        ctx.accounts.share_mint.supply,
        amount_with_fee,
        // This ensures the user provides (burns) enough shares
        Rounding::Up,
    )?;

    // no need to check if user has enough shares
    // since burn would fail in that case
    if shares_to_burn == 0 {
        return Err(VaultProgramError::InsufficientWithdrawAmount.into());
    }

    if shares_to_burn > max_shares {
        return Err(VaultProgramError::SlippageExceeded.into());
    }

    // burn user shares
    ctx.accounts.burn_shares(shares_to_burn)?;

    // pay fee from vault reserve -> fee recipient (if fee > 0)
    if fee > 0 {
        ctx.accounts.transfer_assets_to_fee_recipient(fee)?;
    }

    // transfer from vault to user
    ctx.accounts.transfer_assets_to_user(amount_assets_out)?;

    Ok(())
}

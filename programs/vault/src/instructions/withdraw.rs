use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, burn, Burn, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    error::VaultProgramError,
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
        seeds = [RESERVE_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    /// Vault configuration account (PDA)
    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    /// Fee recipient token account
    #[account(
        mut,
        associated_token::authority = vault.fee_recipient,
        associated_token::mint = asset_mint,
        associated_token::token_program = asset_token_program,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    /// User's asset token account
    #[account(
        mut,
        associated_token::authority = user,
        associated_token::mint = asset_mint,
        associated_token::token_program = asset_token_program,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    /// User's share token account
    #[account(
        mut,
        associated_token::authority = user,
        associated_token::mint = share_mint,
        associated_token::token_program = share_token_program,
    )]
    pub user_shares_account: InterfaceAccount<'info, TokenAccount>,

    pub share_token_program: Interface<'info, TokenInterface>,
    pub asset_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_assets_to_fee_recipient(&mut self, fee: u64) -> Result<()> {
        let asset_mint = self.asset_mint.key();
        let share_mint = self.share_mint.key();

        let cpi_accounts = TransferChecked {
            from: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.fee_recipient.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[
            VAULT_CONFIG_SEED,
            asset_mint.as_ref(),
            share_mint.as_ref(),
            &[self.vault.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.asset_token_program.to_account_info(),
            cpi_accounts,
            seeds,
        );

        token_interface::transfer_checked(cpi_ctx, fee, self.asset_mint.decimals)
    }

    /// Transfers `asset_amount` tokens to the user token account
    pub fn transfer_assets_to_user(&mut self, asset_amount: u64) -> Result<()> {
        let asset_mint = self.asset_mint.key();
        let share_mint = self.share_mint.key();

        let cpi_accounts = TransferChecked {
            from: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.user_assets_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[
            VAULT_CONFIG_SEED,
            asset_mint.as_ref(),
            share_mint.as_ref(),
            &[self.vault.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.asset_token_program.to_account_info(),
            cpi_accounts,
            seeds,
        );

        token_interface::transfer_checked(cpi_ctx, asset_amount, self.asset_mint.decimals)
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

pub fn handler<'info>(ctx: Context<Withdraw>, assets: u64) -> Result<()> {
    require!(!ctx.accounts.vault.paused, VaultProgramError::PausedVault);

    // assets is NET to receiver/user.
    let amount_assets_out = assets;

    // fee computed on the net amount
    let fee = ctx.accounts.vault.get_withdraw_fee(amount_assets_out)?;

    // total assets leaving the vault reserve
    // including the fees
    let amount_with_fee = amount_assets_out
        .checked_add(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    let shares_to_burn = ctx.accounts.vault.get_shares_from_assets(
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

    // burn user shares
    ctx.accounts.burn_shares(shares_to_burn)?;

    // pay fee from vault reserve -> fee recipient (if fee > 0)
    if fee > 0 {
        ctx.accounts.transfer_assets_to_fee_recipient(fee)?;
    }

    // transfer from vault to user
    ctx.accounts.transfer_assets_to_user(amount_assets_out)?;

    // decrease vault assets
    ctx.accounts.vault.decrease_asset_supply(amount_with_fee)?;

    Ok(())
}

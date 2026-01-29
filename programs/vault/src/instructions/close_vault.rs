use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, close_account, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    error::VaultProgramError,
    state::{VaultConfig, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct CloseVault<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: it can be any account to hold SOL
    #[account(mut)]
    pub rent_destination: AccountInfo<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [RESERVE_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        token::mint = asset_mint,
    )]
    pub assets_destination: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CloseVault<'info> {
    pub fn transfer_reserve_assets(&mut self, amount: u64) -> Result<()> {
        let asset_mint = self.asset_mint.key().to_bytes();
        let share_mint = self.share_mint.key().to_bytes();
        let seeds: &[&[u8]] = &[
            VAULT_CONFIG_SEED,
            asset_mint.as_ref(),
            share_mint.as_ref(),
            &[self.vault.bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];
        let vault_transfer_cpi_accounts = TransferChecked {
            from: self.reserve.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.assets_destination.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            vault_transfer_cpi_accounts,
            signer_seeds,
        );
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    pub fn close_reserve_account(&mut self) -> Result<()> {
        let asset_mint = self.asset_mint.key().to_bytes();
        let share_mint = self.share_mint.key().to_bytes();

        let seeds: &[&[u8]] = &[
            VAULT_CONFIG_SEED,
            asset_mint.as_ref(),
            share_mint.as_ref(),
            &[self.vault.bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

        let close_account_cpi_accounts = CloseAccount {
            account: self.reserve.to_account_info(),
            destination: self.rent_destination.clone(),
            authority: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_account_cpi_accounts,
            signer_seeds,
        );
        close_account(cpi_ctx)
    }
}

pub fn handler<'info>(ctx: Context<CloseVault>) -> Result<()> {
    let remaining_amount = ctx.accounts.reserve.amount;
    ctx.accounts.transfer_reserve_assets(remaining_amount)?;
    ctx.accounts.close_reserve_account()?;
    ctx.accounts
        .vault
        .close(ctx.accounts.rent_destination.clone())?;
    Ok(())
}

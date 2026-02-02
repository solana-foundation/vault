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
        close = rent_destination,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CloseVault<'info> {
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
    require!(
        ctx.accounts.share_mint.supply < 1,
        VaultProgramError::MintSupplyShouldBeZero
    );
    require!(
        ctx.accounts.reserve.amount < 1,
        VaultProgramError::VaultShouldBeEmpty
    );
    ctx.accounts.close_reserve_account()?;
    Ok(())
}

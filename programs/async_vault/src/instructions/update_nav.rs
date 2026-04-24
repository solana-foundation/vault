use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use vault_common::VaultProgramError;

use crate::state::{Vault, VAULT_CONFIG_SEED};

#[derive(Accounts)]
pub struct UpdateVaultNav<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler<'info>(ctx: Context<UpdateVaultNav>, updated_nav: u128) -> Result<()> {
    ctx.accounts.vault.nav = updated_nav;
    ctx.accounts.vault.nav_version = ctx
        .accounts
        .vault
        .nav_version
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok(())
}

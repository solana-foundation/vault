use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

use crate::state::Vault;

#[derive(Accounts)]
pub struct UpdateVaultNav<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
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

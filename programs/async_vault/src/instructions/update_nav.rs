use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::Vault};

#[derive(Accounts)]
pub struct UpdateVaultNav<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<UpdateVaultNav>, updated_nav: u128) -> Result<()> {
    ctx.accounts.vault.nav = updated_nav;
    ctx.accounts.vault.nav_version = ctx
        .accounts
        .vault
        .nav_version
        .checked_add(1)
        .ok_or(AsyncVaultError::ArithmeticError)?;
    Ok(())
}

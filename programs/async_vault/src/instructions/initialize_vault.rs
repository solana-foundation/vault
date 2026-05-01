use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::Vault};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    ctx.accounts.vault.initialized = true;

    Ok(())
}

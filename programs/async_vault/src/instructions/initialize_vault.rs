use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Vault, VAULT_CONFIG_SEED},
};

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

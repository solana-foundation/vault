use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
}

/// Marks the async vault as initialized, locking further extension
/// configuration.
///
/// Must be called by the vault authority after all desired extensions
/// have been configured. Once initialized the vault can accept
/// deposits and withdrawals (once unpaused).
///
/// # Errors
///
/// - [`AsyncVaultError::UnauthorizedSigner`] – caller is not the vault authority.
/// - [`AsyncVaultError::VaultAlreadyInitialized`] – vault was already initialized.
pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
    require!(
        !ctx.accounts.vault.initialized,
        AsyncVaultError::VaultAlreadyInitialized
    );

    ctx.accounts.vault.initialized = true;

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct AcceptAuthorityInvitation<'info> {
    pub authority: Signer<'info>,

    pub new_authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<AcceptAuthorityInvitation>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;

    let pending = vault
        .pending_authority
        .ok_or(AsyncVaultError::NoPendingAuthority)?;

    require_keys_eq!(
        ctx.accounts.new_authority.key(),
        pending,
        AsyncVaultError::UnauthorizedSigner
    );

    vault.authority = pending;
    vault.pending_authority = None;
    Ok(())
}

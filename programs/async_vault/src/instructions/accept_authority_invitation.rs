use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::Vault};

#[derive(Accounts)]
pub struct AcceptAuthorityInvitation<'info> {
    pub new_authority: Signer<'info>,

    #[account(mut)]
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

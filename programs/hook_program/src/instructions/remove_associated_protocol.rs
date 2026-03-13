use anchor_lang::prelude::*;

use crate::{
    errors::HookProgramError,
    state::{VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS},
};

#[derive(Accounts)]
pub struct RemoveAssociatedProtocol<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: This is the vault
    pub vault: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS, vault.key().as_ref()],
        bump = vault_associated_protocols.bump,
    )]
    pub vault_associated_protocols: Account<'info, VaultAssociatedProtocols>,

    /// CHECK: This is the protocol to remove
    pub protocol: AccountInfo<'info>,
}

pub fn handler(ctx: Context<RemoveAssociatedProtocol>) -> Result<()> {
    let vap = &mut ctx.accounts.vault_associated_protocols;
    let protocol_key = ctx.accounts.protocol.key();

    let pos = vap
        .protocols
        .iter()
        .position(|p| *p == protocol_key)
        .ok_or(HookProgramError::ProtocolNotFound)?;

    vap.protocols.remove(pos);
    Ok(())
}

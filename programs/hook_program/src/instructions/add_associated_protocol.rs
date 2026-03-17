use anchor_lang::prelude::*;
use vault::state::VaultConfig;

use crate::{
    errors::HookProgramError,
    state::{VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS},
};

// Must match the #[max_len(10)] on VaultAssociatedProtocols::protocols
const MAX_PROTOCOLS: usize = 10;

#[derive(Accounts)]
pub struct AddAssociatedProtocol<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        has_one = authority @ HookProgramError::UnauthorizedAuthority,
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS, vault.key().as_ref()],
        bump = vault_associated_protocols.bump,
    )]
    pub vault_associated_protocols: Account<'info, VaultAssociatedProtocols>,

    /// CHECK: This is the protocol to associate
    pub protocol: AccountInfo<'info>,
}

pub fn handler(ctx: Context<AddAssociatedProtocol>) -> Result<()> {
    let vap = &mut ctx.accounts.vault_associated_protocols;
    let protocol_key = ctx.accounts.protocol.key();

    require!(
        vap.protocols.len() < MAX_PROTOCOLS,
        HookProgramError::MaxProtocolsReached
    );

    require!(
        !vap.protocols.contains(&protocol_key),
        HookProgramError::ProtocolAlreadyAssociated
    );

    vap.protocols.push(protocol_key);
    Ok(())
}

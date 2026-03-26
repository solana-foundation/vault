use anchor_lang::prelude::*;
use vault::state::VaultConfig;

use crate::{
    errors::HookProgramError,
    state::{
        AssociatedProtocol, VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS_SEED,
        VAULT_PROTOCOL_DEPOSIT_SEED,
    },
};

#[derive(Accounts)]
pub struct RemoveAssociatedProtocol<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        has_one = authority @ HookProgramError::UnauthorizedAuthority,
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, vault.share_mint_address.key().as_ref()],
        bump = vault_associated_protocols.bump,
    )]
    pub vault_associated_protocols: Account<'info, VaultAssociatedProtocols>,

    /// CHECK: This is the protocol to remove
    pub protocol: AccountInfo<'info>,

    #[account(
        mut,
        close = authority,
        seeds = [
            VAULT_PROTOCOL_DEPOSIT_SEED,
            vault.share_mint_address.key().as_ref(),
            protocol.key().as_ref(),
        ],
        bump = associated_protocol.bump,
    )]
    pub associated_protocol: Account<'info, AssociatedProtocol>,
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

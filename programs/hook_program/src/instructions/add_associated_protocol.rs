use anchor_lang::prelude::*;
use vault::state::Vault;

use crate::{
    errors::HookProgramError,
    state::{
        AssociatedProtocol, VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS_SEED,
        VAULT_PROTOCOL_DEPOSIT_SEED,
    },
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
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, vault.share_mint_address.key().as_ref()],
        bump = vault_associated_protocols.bump,
    )]
    pub vault_associated_protocols: Account<'info, VaultAssociatedProtocols>,

    /// CHECK: This is the protocol to associate
    pub protocol: AccountInfo<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + AssociatedProtocol::INIT_SPACE,
        seeds = [
            VAULT_PROTOCOL_DEPOSIT_SEED,
            vault.share_mint_address.key().as_ref(),
            protocol.key().as_ref(),
        ],
        bump,
    )]
    pub associated_protocol: Account<'info, AssociatedProtocol>,

    /// CHECK: Token account belonging to the protocol that tracks its deposits
    pub token_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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

    let ap = &mut ctx.accounts.associated_protocol;
    ap.vault = ctx.accounts.vault.key();
    ap.protocol = protocol_key;
    ap.token_account = ctx.accounts.token_account.key();
    ap.bump = ctx.bumps.associated_protocol;

    Ok(())
}

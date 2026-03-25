use anchor_lang::prelude::*;
use vault::state::VaultConfig;

use crate::{
    errors::HookProgramError,
    state::{VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS_SEED},
};

#[derive(Accounts)]
pub struct InitVaultAssociatedProtocols<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        has_one = authority @ HookProgramError::UnauthorizedAuthority,
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + VaultAssociatedProtocols::INIT_SPACE,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, vault.share_mint_address.key().as_ref()],
        bump
    )]
    pub vault_associated_protocols: Account<'info, VaultAssociatedProtocols>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitVaultAssociatedProtocols>) -> Result<()> {
    let vap = &mut ctx.accounts.vault_associated_protocols;
    vap.protocols = Vec::new();
    vap.vault = ctx.accounts.vault.key();
    vap.bump = ctx.bumps.vault_associated_protocols;
    Ok(())
}

use anchor_lang::prelude::*;

use crate::state::{VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS};

#[derive(Accounts)]
pub struct InitVaultAssociatedProtocols<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: This is the vault
    pub vault: AccountInfo<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + VaultAssociatedProtocols::INIT_SPACE,
        seeds = [VAULT_ASSOCIATED_PROTOCOLS, vault.key().as_ref()],
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

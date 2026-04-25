use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct SetOperator<'info> {
    pub authority: Signer<'info>,

    pub operator: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<SetOperator>) -> Result<()> {
    ctx.accounts.vault.operator = Some(ctx.accounts.operator.key());
    Ok(())
}

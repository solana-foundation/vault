use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateVaultArgs {
    paused: Option<bool>,
    async_inflows: Option<bool>,
    async_outflows: Option<bool>,
}

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
    let vault = &mut ctx.accounts.vault;

    vault.paused = args.paused.unwrap_or(vault.paused);
    vault.async_inflows = args.async_inflows.unwrap_or(vault.async_inflows);
    vault.async_outflows = args.async_outflows.unwrap_or(vault.async_outflows);

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct ApproveRequest<'info> {
    pub authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        has_one = asset_mint @ AsyncVaultError::InvalidAssetMint,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        has_one = vault @ AsyncVaultError::InvalidRequest,
    )]
    pub request: Account<'info, Request>,
}

pub fn handler<'info>(ctx: Context<ApproveRequest>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    let vault = &mut ctx.accounts.vault;
    let request = &mut ctx.accounts.request;

    // Update Request state to be Claimable and pin price to Vault's current NAV
    request.price = vault.nav;
    request.request_state = RequestState::Claimable;

    // Decrement Vault's pending_async_requests count
    vault.pending_async_requests = vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(AsyncVaultError::ArithmeticError)?;

    Ok(())
}

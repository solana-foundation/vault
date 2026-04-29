use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, Vault},
};

#[derive(Accounts)]
pub struct ApproveRequest<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
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

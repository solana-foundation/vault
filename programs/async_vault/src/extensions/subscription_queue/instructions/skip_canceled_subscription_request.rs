use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::subscription_queue::processor::check_and_advance_subscription_queue,
    state::{Request, RequestState, RequestType, Vault},
};

#[derive(Accounts)]
pub struct SkipCanceledSubscriptionRequest<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        close = owner,
        constraint = request.request_state == RequestState::Canceled @ AsyncVaultError::RequestIsNotCanceled,
        has_one = vault,
    )]
    pub request: Account<'info, Request>,

    /// CHECK: receives rent from the closed request account; must be the original request owner
    #[account(
        mut,
        constraint = owner.key() == request.owner @ AsyncVaultError::UnauthorizedSigner
    )]
    pub owner: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Advances the subscription queue past a canceled deposit request, then closes the request
/// account and returns rent to the original owner. Permissionless — anyone can call this to
/// unblock the queue. Callers must invoke this in ascending ID order for consecutive tombstones.
pub fn handler(ctx: Context<SkipCanceledSubscriptionRequest>) -> Result<()> {
    check_and_advance_subscription_queue(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.request.to_account_info(),
    )
}

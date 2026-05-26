use anchor_lang::prelude::*;

use crate::{
    extensions::{
        fifo_queues::check_and_advance_queue,
        redemption_queue::processor::{RedemptionQueue, RedemptionQueueRequest},
        subscription_queue::processor::{SubscriptionQueue, SubscriptionQueueRequest},
    },
    state::{Request, RequestState, RequestType, Vault},
};

#[derive(Accounts)]
pub struct SkipCanceledQueueRequest<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        close = owner,
        constraint = request.request_state == RequestState::Canceled @ crate::error::AsyncVaultError::RequestIsNotCanceled,
        has_one = vault,
        has_one = owner,
    )]
    pub request: Account<'info, Request>,

    /// CHECK: receives rent from the closed request account; must be the original request owner
    #[account(mut)]
    pub owner: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Advances the queue past a canceled request, closes the request account, and returns rent to
/// the original owner. Permissionless — anyone can call this to unblock the queue. Works for
/// both subscription and redemption queues; the queue type is inferred from the request type.
/// Callers must invoke this in ascending ID order for consecutive tombstones.
pub fn handler(ctx: Context<SkipCanceledQueueRequest>) -> Result<()> {
    match ctx.accounts.request.request_type {
        RequestType::Deposit => {
            check_and_advance_queue::<SubscriptionQueue, SubscriptionQueueRequest>(
                &ctx.accounts.vault.to_account_info(),
                &ctx.accounts.request.to_account_info(),
            )
        }
        RequestType::Redeem => check_and_advance_queue::<RedemptionQueue, RedemptionQueueRequest>(
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.request.to_account_info(),
        ),
    }
}

use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{init_vault_extension, VaultExtension},
    state::Vault,
};

use super::super::processor::SubscriptionQueue;

#[derive(Accounts)]
pub struct InitializeSubscriptionQueue<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + SubscriptionQueue::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

/// Adds the SubscriptionQueue TLV extension to the vault, initializing both counters
/// to zero. Must be called before vault initialization.
pub fn handler(ctx: Context<InitializeSubscriptionQueue>) -> Result<()> {
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &SubscriptionQueue {
            all_time_total_subscription_requests: 0,
            last_processed_subscription_request_index: 0,
        },
    )
}

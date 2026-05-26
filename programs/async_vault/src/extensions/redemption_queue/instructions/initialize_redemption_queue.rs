use anchor_lang::prelude::*;

use bytemuck::Zeroable;

use crate::{
    error::AsyncVaultError,
    extensions::{init_vault_extension, VaultExtension},
    state::Vault,
};

use super::super::processor::RedemptionQueue;

#[derive(Accounts)]
pub struct InitializeRedemptionQueue<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + RedemptionQueue::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

/// Adds the RedemptionQueue TLV extension to the vault, initializing both counters
/// to zero. Must be called before vault initialization.
pub fn handler(ctx: Context<InitializeRedemptionQueue>) -> Result<()> {
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &RedemptionQueue::zeroed(),
    )
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    extensions::{
        request_extensions::has_request_extension,
        subscription_queue::processor::SubscriptionQueueRequest,
    },
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct CancelQueuedDepositRequest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = asset_mint @ AsyncVaultError::InvalidAssetMint,
        has_one = share_mint @ AsyncVaultError::InvalidShareMint,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        constraint = request.owner == user.key() @ AsyncVaultError::UnauthorizedSigner,
        constraint = request.request_type == RequestType::Deposit @ AsyncVaultError::InvalidRequestType,
        has_one = vault,
    )]
    pub request: Account<'info, Request>,

    #[account(
        mut,
        token::mint = asset_mint.key(),
        token::authority = user
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = asset_mint.key(),
        token::authority = vault,
        token::token_program = asset_token_program,
        constraint = vault.pending_vault.key() == asset_pending_vault.key() @ AsyncVaultError::InvalidPendingVault
    )]
    pub asset_pending_vault: InterfaceAccount<'info, TokenAccount>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CancelQueuedDepositRequest<'info> {
    /// Transfers deposited assets from the pending vault back to the user's token account,
    /// using the vault's PDA authority to sign the CPI transfer.
    pub fn transfer_assets_to_user(&self, amount: u64) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: self.asset_pending_vault.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.user_token_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx =
            CpiContext::new_with_signer(self.asset_token_program.to_account_info(), cpi_accounts, seeds);
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }
}

/// Cancels a pending queued deposit request. Assets are refunded immediately. The request
/// account remains open as a tombstone so the subscription queue can advance past it via
/// `skip_canceled_subscription_request`.
pub fn handler(ctx: Context<CancelQueuedDepositRequest>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    require!(
        ctx.accounts.request.request_state == RequestState::Pending,
        AsyncVaultError::RequestIsNotPending,
    );
    let request_info = ctx.accounts.request.to_account_info();
    let request_data = request_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let has_queue_ext = has_request_extension::<SubscriptionQueueRequest>(&request_data);
    require!(has_queue_ext, AsyncVaultError::UninitializedExtension);
    drop(request_data);

    let refund_amount = ctx.accounts.request.amount;
    ctx.accounts.transfer_assets_to_user(refund_amount)?;

    ctx.accounts.request.request_state = RequestState::Canceled;
    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    Ok(())
}

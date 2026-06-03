use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface};

use crate::{
    error::AsyncVaultError,
    extensions::request_extensions::has_request_extension,
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

use super::super::processor::RedemptionQueueRequest;

#[derive(Accounts)]
pub struct CancelQueuedRedemptionRequest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
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
        constraint = request.request_type == RequestType::Redeem @ AsyncVaultError::InvalidRequestType,
        has_one = vault,
    )]
    pub request: Account<'info, Request>,

    #[account(
        mut,
        token::mint = share_mint.key(),
        token::authority = user,
        token::token_program = share_token_program,
    )]
    pub user_share_account: InterfaceAccount<'info, TokenAccount>,

    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CancelQueuedRedemptionRequest<'info> {
    /// Mints burned shares back to the user's share account using the vault PDA as authority.
    pub fn mint_shares_to_user(&self, amount: u64) -> Result<()> {
        let cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: self.user_share_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx = CpiContext::new_with_signer(
            self.share_token_program.to_account_info(),
            cpi_accounts,
            seeds,
        );
        token_interface::mint_to(cpi_ctx, amount)
    }

    /// Validate that the RedemptionQueueRequest extension is present on the request.
    pub fn validate_has_redemption_queue_extension(&self) -> Result<()> {
        let request_info = self.request.to_account_info();
        let request_data = request_info
            .data
            .try_borrow()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let has_queue_ext = has_request_extension::<RedemptionQueueRequest>(&request_data);
        require!(has_queue_ext, AsyncVaultError::UninitializedExtension);
        Ok(())
    }
}

/// Cancels a pending queued redeem request. Shares are minted back to the user immediately.
/// The request account remains open as a tombstone so the redemption queue can advance past
/// it via `skip_canceled_queue_request`.
pub fn handler(ctx: Context<CancelQueuedRedemptionRequest>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    require!(
        ctx.accounts.request.request_state == RequestState::Pending,
        AsyncVaultError::RequestIsNotPending,
    );
    ctx.accounts.validate_has_redemption_queue_extension()?;

    let refund_shares = ctx.accounts.request.amount;
    ctx.accounts.mint_shares_to_user(refund_shares)?;

    ctx.accounts.request.request_state = RequestState::Canceled;
    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(AsyncVaultError::ArithmeticError)?;

    Ok(())
}

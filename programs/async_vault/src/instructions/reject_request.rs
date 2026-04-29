use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct RejectRequest<'info> {
    pub authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = request.vault == vault.key(),
        close = user,
    )]
    pub request: Account<'info, Request>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
        constraint = vault.authority == authority.key() @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: Validated against request.owner. Receives rent on account close.
    #[account(
        mut,
        constraint = user.key() == request.owner @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
        constraint = vault.pending_vault == pending_vault.key() @ AsyncVaultError::InvalidPendingVault,
    )]
    pub pending_vault: InterfaceAccount<'info, TokenAccount>,

    pub asset_token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<RejectRequest>) -> Result<()> {
    require!(
        ctx.accounts
            .request
            .request_state
            .eq(&RequestState::Pending),
        AsyncVaultError::RequestInvalidState
    );
    let refund_amount = ctx
        .accounts
        .request
        .amount
        .checked_add(ctx.accounts.request.fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    let share_mint = ctx.accounts.share_mint.key();
    let seeds: &[&[&[u8]]] = &[&[
        VAULT_CONFIG_SEED,
        share_mint.as_ref(),
        &[ctx.accounts.vault.bump],
    ]];

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.pending_vault.to_account_info(),
        mint: ctx.accounts.asset_mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.asset_token_program.to_account_info(),
        cpi_accounts,
        seeds,
    );
    token_interface::transfer_checked(cpi_ctx, refund_amount, ctx.accounts.asset_mint.decimals)?;

    ctx.accounts.request.request_state = RequestState::Rejected;

    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
    utils::{calculate_assets, calculate_shares},
};

#[derive(Accounts)]
pub struct ApproveRequest<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        has_one = asset_mint @ AsyncVaultError::InvalidAssetMint,
        has_one = share_mint @ AsyncVaultError::InvalidShareMint,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        has_one = vault @ AsyncVaultError::InvalidRequest,
    )]
    pub request: Account<'info, Request>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = vault.vault_token_account == vault_token_account.key(),
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault.pending_vault == pending_vault.key() @ AsyncVaultError::InvalidPendingVault,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
    )]
    pub pending_vault: InterfaceAccount<'info, TokenAccount>,

    pub asset_token_program: Interface<'info, TokenInterface>,
}

impl<'info> ApproveRequest<'info> {
    /// Transfers assets from pending vault to vault, enabling the Authority to withdraw
    /// in a future transaction.
    pub fn settle_deposit(&self, seeds: &[&[&[u8]]], amount: u64) -> Result<()> {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                TransferChecked {
                    from: self.pending_vault.to_account_info(),
                    mint: self.asset_mint.to_account_info(),
                    to: self.vault_token_account.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            amount,
            self.asset_mint.decimals,
        )
    }

    /// Transfers assets from vault to pending vault, removing them from the supply
    /// that the Authority may withdraw from.
    pub fn settle_redeem(&self, seeds: &[&[&[u8]]], assets: u64) -> Result<()> {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                TransferChecked {
                    from: self.vault_token_account.to_account_info(),
                    mint: self.asset_mint.to_account_info(),
                    to: self.pending_vault.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            assets,
            self.asset_mint.decimals,
        )
    }
}

// TODO add fee handling
pub fn handler(ctx: Context<ApproveRequest>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    require!(
        matches!(ctx.accounts.request.request_state, RequestState::Pending),
        AsyncVaultError::RequestNotPending
    );
    require!(ctx.accounts.vault.nav > 0, VaultProgramError::NavIsNotSet);

    let nav = ctx.accounts.vault.nav;
    let decimals = ctx.accounts.share_mint.decimals;
    let share_mint_key = ctx.accounts.share_mint.key();
    let vault_bump = ctx.accounts.vault.bump;
    let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint_key.as_ref(), &[vault_bump]]];

    let is_deposit = matches!(ctx.accounts.request.request_type, RequestType::Deposit);
    let original_amount = ctx.accounts.request.amount;

    // Transfer assets between Vault and Pending Vault (aka escrow)
    let claimable_amount = if is_deposit {
        let shares = calculate_shares(nav, decimals, original_amount)?;
        ctx.accounts.settle_deposit(seeds, original_amount)?;
        shares
    } else {
        let assets = calculate_assets(nav, decimals, original_amount)?;
        ctx.accounts.settle_redeem(seeds, assets)?;
        assets
    };

    let vault = &mut ctx.accounts.vault;
    let request = &mut ctx.accounts.request;

    // Upate Vault's `total_asset_balance`
    if is_deposit {
        vault.total_asset_balance = vault
            .total_asset_balance
            .checked_add(original_amount)
            .ok_or(AsyncVaultError::ArithmeticError)?;
    } else {
        vault.total_asset_balance = vault
            .total_asset_balance
            .checked_sub(claimable_amount)
            .ok_or(AsyncVaultError::ArithmeticError)?;
    }

    // Update Request's amount with the claimable amount
    request.amount = claimable_amount;
    request.price = nav;
    request.request_state = RequestState::Claimable;

    // Decrement Vault's pending Requests
    vault.pending_async_requests = vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(AsyncVaultError::ArithmeticError)?;

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    error::AsyncVaultError,
    extensions::{
        fifo_queue::check_and_advance_queue,
        redemption_queue::processor::{RedemptionQueue, RedemptionQueueRequest},
        subscription_queue::processor::{SubscriptionQueue, SubscriptionQueueRequest},
    },
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct RejectRequest<'info> {
    pub authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = asset_mint @ AsyncVaultError::InvalidAssetMint,
        has_one = share_mint @ AsyncVaultError::InvalidShareMint,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
        constraint = vault.authority == authority.key() @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        close = user,
        constraint = request.owner == user.key() @ AsyncVaultError::UnauthorizedSigner,
        has_one = vault,
    )]
    pub request: Box<Account<'info, Request>>,

    /// CHECK: Validated against request.owner. Receives rent on account close.
    #[account(mut)]
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = asset_mint.key(),
        token::authority = user
    )]
    pub user_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = asset_mint.key(),
        token::authority = vault,
        token::token_program = asset_token_program,
        constraint = vault.pending_vault.key() == asset_pending_vault.key() @ AsyncVaultError::InvalidPendingVault
    )]
    pub asset_pending_vault: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = share_mint.key(),
        token::authority = user,
        token::token_program = share_token_program,
    )]
    pub user_share_account: Option<InterfaceAccount<'info, TokenAccount>>,

    pub share_token_program: Option<Interface<'info, TokenInterface>>,
    pub asset_token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

impl<'info> RejectRequest<'info> {
    #[inline(never)]
    pub fn transfer_assets_to_user(&self, amount: u64) -> Result<()> {
        let pending_vault = self
            .asset_pending_vault
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;
        let user_token_account = self
            .user_token_account
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;
        let asset_token_program = self
            .asset_token_program
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;

        let cpi_accounts = TransferChecked {
            from: pending_vault.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: user_token_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx =
            CpiContext::new_with_signer(asset_token_program.key(), cpi_accounts, seeds);
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    /// Enforces FIFO ordering for queued deposit and redeem requests.
    #[inline(never)]
    pub fn check_fifo_ordering(&self) -> Result<()> {
        if matches!(self.request.request_type, RequestType::Deposit) {
            check_and_advance_queue::<SubscriptionQueue, SubscriptionQueueRequest>(
                &self.vault.to_account_info(),
                &self.request.to_account_info(),
            )?;
        }
        if matches!(self.request.request_type, RequestType::Redeem) {
            check_and_advance_queue::<RedemptionQueue, RedemptionQueueRequest>(
                &self.vault.to_account_info(),
                &self.request.to_account_info(),
            )?;
        }
        Ok(())
    }

    #[inline(never)]
    pub fn mint_shares(&self, amount: u64) -> Result<()> {
        let user_share_account = self
            .user_share_account
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;
        let share_token_program = self
            .share_token_program
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;

        let cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: user_share_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx =
            CpiContext::new_with_signer(share_token_program.key(), cpi_accounts, seeds);
        token_interface::mint_to(cpi_ctx, amount)
    }
}

pub fn handler(ctx: Context<RejectRequest>) -> Result<()> {
    require!(
        ctx.accounts
            .request
            .request_state
            .eq(&RequestState::Pending),
        AsyncVaultError::RequestInvalidState
    );

    ctx.accounts.check_fifo_ordering()?;

    match ctx.accounts.request.request_type {
        RequestType::Deposit => {
            let refund_amount = ctx.accounts.request.amount;
            ctx.accounts.transfer_assets_to_user(refund_amount)?;
        }
        RequestType::Redeem => {
            let shares = ctx.accounts.request.amount;
            ctx.accounts.mint_shares(shares)?;
        }
    }

    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(AsyncVaultError::ArithmeticError)?;

    Ok(())
}

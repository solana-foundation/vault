use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct CancelRequest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        close = user,
        constraint = request.owner == user.key() @ AsyncVaultError::UnauthorizedSigner,
        constraint = request.vault == vault.key() @ AsyncVaultError::InvalidPendingVault,
    )]
    pub request: Account<'info, Request>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,

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
        token::authority = vault,
        token::token_program = share_token_program,
    )]
    pub share_pending_vault: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = share_mint.key(),
        token::authority = user
    )]
    pub user_share_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    pub share_token_program: Option<Interface<'info, TokenInterface>>,
    pub asset_token_program: Option<Interface<'info, TokenInterface>>,
}

impl<'info> CancelRequest<'info> {
    pub fn transfer_assets_to_user(&self, amount: u64) -> Result<()> {
        let asset_pending_vault = self
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
            from: asset_pending_vault.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: user_token_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx =
            CpiContext::new_with_signer(asset_token_program.to_account_info(), cpi_accounts, seeds);
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    pub fn transfer_shares_to_user(&self, amount: u64) -> Result<()> {
        let share_pending_vault = self
            .share_pending_vault
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;
        let user_share_token_account = self
            .user_share_token_account
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;
        let share_token_program = self
            .share_token_program
            .as_ref()
            .ok_or(error!(AsyncVaultError::MissingRequiredAccount))?;

        let cpi_accounts = TransferChecked {
            from: share_pending_vault.to_account_info(),
            mint: self.share_mint.to_account_info(),
            to: user_share_token_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let share_mint = self.share_mint.key();
        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let cpi_ctx =
            CpiContext::new_with_signer(share_token_program.to_account_info(), cpi_accounts, seeds);
        token_interface::transfer_checked(cpi_ctx, amount, self.share_mint.decimals)
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, CancelRequest<'info>>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    require!(
        ctx.accounts.request.request_state == RequestState::Pending,
        AsyncVaultError::RequestIsNotPending,
    );
    match ctx.accounts.request.request_type {
        RequestType::Deposit => {
            let refund_amount = ctx
                .accounts
                .request
                .amount
                .checked_add(ctx.accounts.request.fee)
                .ok_or(VaultProgramError::ArithmeticError)?;

            ctx.accounts.transfer_assets_to_user(refund_amount)?;
        }
        RequestType::Redeem => {
            let shares = ctx.accounts.request.amount;
            ctx.accounts.transfer_shares_to_user(shares)?;
        }
    }
    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    Ok(())
}

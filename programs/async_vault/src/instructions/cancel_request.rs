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
        close = user
    )]
    pub request: Account<'info, Request>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

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
        constraint = vault.pending_vault.key() == pending_vault.key() @ AsyncVaultError::InvalidPendingVault
    )]
    pub pending_vault: InterfaceAccount<'info, TokenAccount>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> CancelRequest<'info> {
    /// Generic asset token transfer. Automatically uses vault PDA signing when
    /// authority is the vault, otherwise performs a plain user-signed transfer.
    pub fn transfer_asset_token(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from,
            mint: self.asset_mint.to_account_info(),
            to,
            authority: authority.clone(),
        };

        if authority.key() == self.vault.key() {
            let share_mint = self.share_mint.key();
            let seeds: &[&[&[u8]]] =
                &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
            let cpi_ctx = CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                cpi_accounts,
                seeds,
            );
            token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
        } else {
            let cpi_ctx = CpiContext::new(self.asset_token_program.to_account_info(), cpi_accounts);
            token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
        }
    }
}
pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, CancelRequest<'info>>) -> Result<()> {
    // If vault is paused can you cancel the request?
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    require!(
        ctx.accounts
            .request
            .request_state
            .eq(&RequestState::Pending),
        AsyncVaultError::RequestIsNotPending,
    );

    let amount = ctx.accounts.request.amount;
    // Transfer assets from user into pending vault
    ctx.accounts.transfer_asset_token(
        ctx.accounts.pending_vault.to_account_info(),
        ctx.accounts.user_token_account.to_account_info(),
        ctx.accounts.user.to_account_info(),
        amount,
    )?;

    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_sub(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    Ok(())
}

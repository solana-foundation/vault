use crate::error::AsyncVaultError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};
use vault_common::VaultProgramError;

use crate::state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED};

use super::create_deposit_request::RequestArgs;

#[derive(Accounts)]
pub struct CreateRedeemRequest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        space = 8 + Request::INIT_SPACE,
        payer = user,
    )]
    pub request: Account<'info, Request>,

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
        token::mint = share_mint.key(),
        token::authority = user
    )]
    pub user_share_account: InterfaceAccount<'info, TokenAccount>,

    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateRedeemRequest<'info> {
    pub fn burn_shares(&self, amount: u64) -> Result<()> {
        let cpi_accounts = Burn {
            mint: self.share_mint.to_account_info(),
            from: self.user_share_account.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.share_token_program.to_account_info(), cpi_accounts);
        token_interface::burn(cpi_ctx, amount)
    }
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateRedeemRequest<'info>>,
    args: RequestArgs,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    require!(
        ctx.accounts.vault.async_outflows,
        VaultProgramError::AsyncOutflowsDisabled
    );

    require!(args.amount > 0, VaultProgramError::InsufficientRedeemAmount);

    ctx.accounts.burn_shares(args.amount)?;

    let gross_assets = ctx
        .accounts
        .vault
        .calculate_assets(ctx.accounts.share_mint.decimals, args.amount)?;

    require!(gross_assets > 0, VaultProgramError::ZeroAssets);

    let current_timestamp = Clock::get()?.unix_timestamp;
    ctx.accounts.request.set_inner(Request {
        vault: ctx.accounts.vault.key(),
        request_type: RequestType::Redeem,
        request_state: RequestState::Pending,
        owner: ctx.accounts.user.key(),
        amount: args.amount,
        price: ctx.accounts.vault.nav,
        asset_mint_address: ctx.accounts.asset_mint.key(),
        created_at: current_timestamp,
        nav_update_version: ctx.accounts.vault.nav_version,
        operator: args.operator,
    });

    ctx.accounts.vault.pending_async_requests = ctx
        .accounts
        .vault
        .pending_async_requests
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    Ok(())
}

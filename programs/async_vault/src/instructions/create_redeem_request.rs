use crate::error::AsyncVaultError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};
use vault_common::VaultProgramError;

use crate::{
    extensions::{
        self,
        fifo_queue::next_queue_request_id,
        redemption_queue::processor::{RedemptionQueue, RedemptionQueueRequest},
        request_extensions::{compute_request_extension_space, init_request_extension},
    },
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

use super::create_deposit_request::RequestArgs;

#[derive(Accounts)]
pub struct CreateRedeemRequest<'info> {
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

    // Space is extended conditionally: if RedemptionQueue is active on the vault,
    // extra bytes are reserved for the RedemptionQueueRequest TLV extension.
    #[account(
        init,
        space = 8 + Request::INIT_SPACE + compute_request_extension_space(&vault.to_account_info()),
        payer = user,
    )]
    pub request: Account<'info, Request>,

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
    /// Burn User shares
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

pub fn handler(ctx: Context<CreateRedeemRequest>, args: RequestArgs) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    // Extension: PausableRedemption handling
    extensions::pausable_redemptions::check_redemptions_paused(
        &ctx.accounts.vault.to_account_info(),
    )?;

    require!(args.amount > 0, VaultProgramError::InsufficientRedeemAmount);

    ctx.accounts.burn_shares(args.amount)?;

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

    // Extension: RedemptionQueue — increment counter and tag the request with its ID.
    if let Some(id) =
        next_queue_request_id::<RedemptionQueue>(&ctx.accounts.vault.to_account_info())?
    {
        init_request_extension(
            &ctx.accounts.request.to_account_info(),
            &RedemptionQueueRequest { id },
        )?;
    }

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    error::AsyncVaultError,
    state::{Request, RequestState, RequestType, Vault, VAULT_CONFIG_SEED},
};

// TODO make token accounts optional such that the accounts needed for deposit/redeem
// branches are only present in the respective branch.

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
        has_one = asset_mint @ AsyncVaultError::InvalidAssetMint,
        has_one = share_mint @ AsyncVaultError::InvalidShareMint,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        close = user,
        has_one = vault @ AsyncVaultError::InvalidRequest,
    )]
    pub request: Account<'info, Request>,

    /// Reserve — destination for Deposit assets; source for Redeem assets
    #[account(
        mut,
        constraint = vault.vault_token_account == vault_token_account.key(),
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Pending deposit vault — source for Deposit claims
    #[account(
        mut,
        constraint = vault.pending_vault == pending_vault.key() @ AsyncVaultError::InvalidPendingVault,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
    )]
    pub pending_vault: InterfaceAccount<'info, TokenAccount>,

    /// User's share TokenAccount — receives minted shares on Deposit (must be owned by
    /// request.owner)
    #[account(
        mut,
        token::mint = share_mint,
        token::authority = request.owner,
        token::token_program = share_token_program,
    )]
    pub user_share_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's asset TokenAccount — receives transferred assets on Redeem (must be owned by
    /// request.owner)
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = request.owner,
        token::token_program = asset_token_program,
    )]
    pub user_asset_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
}

impl<'info> Claim<'info> {
    /// Transfers `amount` assets from the pending vault into the reserve, then mints
    /// `shares` to the user's share account.
    pub fn deposit(
        &self,
        seeds: &[&[&[u8]]],
        amount: u64,
        shares: u64,
    ) -> Result<()> {
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
        )?;

        token_interface::mint_to(
            CpiContext::new_with_signer(
                self.share_token_program.to_account_info(),
                MintTo {
                    mint: self.share_mint.to_account_info(),
                    to: self.user_share_account.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            shares,
        )
    }

    /// Transfers `assets` from the vault reserve to the user's asset account.
    pub fn redeem(&self, seeds: &[&[&[u8]]], assets: u64) -> Result<()> {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                TransferChecked {
                    from: self.vault_token_account.to_account_info(),
                    mint: self.asset_mint.to_account_info(),
                    to: self.user_asset_account.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            assets,
            self.asset_mint.decimals,
        )
    }
}

pub fn handler(ctx: Context<Claim>) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    let request = &ctx.accounts.request;
    let user_key = ctx.accounts.user.key();

    require!(
        user_key == request.owner || request.operator.map_or(false, |op| user_key == op),
        AsyncVaultError::UnauthorizedSigner
    );

    require!(
        matches!(request.request_state, RequestState::Claimable),
        AsyncVaultError::RequestNotClaimable
    );

    let amount = request.amount;
    let decimals = ctx.accounts.share_mint.decimals;
    let share_mint_key = ctx.accounts.share_mint.key();
    let vault_bump = ctx.accounts.vault.bump;
    let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint_key.as_ref(), &[vault_bump]]];

    match request.request_type {
        RequestType::Deposit => {
            // TODO calculate fees before shares, if DepositFee extension is enabled
            let shares = ctx.accounts.request.calculate_shares(decimals, amount)?;
            ctx.accounts.deposit(seeds, amount, shares)?;
        }
        RequestType::Redeem => {
            let assets = ctx.accounts.request.calculate_assets(decimals, amount)?;
            // TODO calculate fees and transfer if WithdrawFees are enabled
            ctx.accounts.redeem(seeds, assets)?;
        }
    }

    Ok(())
}

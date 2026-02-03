use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, mint_to, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    error::VaultProgramError,
    state::{Rounding, VaultConfig, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [RESERVE_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        associated_token::authority = vault.fee_recipient,
        associated_token::mint = asset_mint,
        associated_token::token_program = reserve_token_program,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::authority = user,
        associated_token::mint = asset_mint,
        associated_token::token_program = reserve_token_program,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::authority = user,
        associated_token::mint = share_mint,
        associated_token::token_program = token_program,
    )]
    pub user_shares_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub reserve_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn transfer_reserve_token_fee_to_fee_recipient(&mut self, fee: u64) -> Result<()> {
        let fee_recipient_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.fee_recipient.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            self.reserve_token_program.to_account_info(),
            fee_recipient_transfer_cpi_accounts,
        );

        token_interface::transfer_checked(cpi_ctx, fee, self.asset_mint.decimals)
    }

    pub fn transfer_reserve_token_to_vault(&mut self, amount: u64) -> Result<()> {
        let vault_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.reserve.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.reserve_token_program.to_account_info(),
            vault_transfer_cpi_accounts,
        );
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    pub fn mint(&mut self, amount: u64) -> Result<()> {
        let asset_mint = self.asset_mint.key();
        let share_mint = self.share_mint.key();
        let mint_to_cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: self.user_shares_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[
            VAULT_CONFIG_SEED,
            asset_mint.as_ref(),
            share_mint.as_ref(),
            &[self.vault.bump],
        ]];

        let mint_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            mint_to_cpi_accounts,
            seeds,
        );
        mint_to(mint_cpi_ctx, amount)
    }
}
pub fn handler<'info>(ctx: Context<Deposit>, assets: u64) -> Result<()> {
    require!(!ctx.accounts.vault.paused, VaultProgramError::PausedVault);
    let fee = ctx.accounts.vault.get_deposit_fee(assets)?;
    let expected_new_total_asset_balance = ctx
        .accounts
        .vault
        .total_asset_balance
        .checked_add(assets)
        .ok_or(VaultProgramError::ArithmeticError)?
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        expected_new_total_asset_balance <= ctx.accounts.vault.vault_asset_cap,
        VaultProgramError::MaxVaultAssetCapExceeded
    );

    let remaining_amount = assets
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    let shares = ctx.accounts.vault.get_shares_from_assets(
        &ctx.accounts.share_mint,
        remaining_amount,
        Rounding::Down,
    )?;

    if shares == 0 {
        return Err(VaultProgramError::InsufficientDepositAmount.into());
    }

    ctx.accounts.vault.increase_asset_supply(remaining_amount)?;
    ctx.accounts
        .transfer_reserve_token_fee_to_fee_recipient(fee)?;
    ctx.accounts
        .transfer_reserve_token_to_vault(remaining_amount)?;
    ctx.accounts.mint(shares)?;
    Ok(())
}

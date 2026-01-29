use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{mint_to, MintTo, TransferChecked},
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::state::{VaultConfig, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED};

#[derive(Accounts)]
pub struct MintNewStable<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [RESERVE_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        associated_token::authority = vault.fee_recipient,
        associated_token::mint = asset_mint,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = asset_mint,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = share_mint,
    )]
    pub user_shares_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub reserve_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> MintNewStable<'info> {
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
    pub fn mint_new_stable(&mut self, amount: u64) -> Result<()> {
        let mint = self.share_mint.key();
        let mint_to_cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: self.user_shares_account.to_account_info(),
            authority: self.mint_authority.to_account_info(),
        };

        // We need the mint authority ... it could be a PDA?

        let mint_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            mint_to_cpi_accounts,
            seeds,
        );
        mint_to(mint_cpi_ctx, amount)
    }
}

pub fn handler<'info>(ctx: Context<MintNewStable>, shares: u64) -> Result<()> {
    let assets = 0;
    let fee = ctx.accounts.vault.get_deposit_fee(assets);
    ctx.accounts.vault.increase_asset_supply(assets)?;
    ctx.accounts.transfer_reserve_token_to_vault(assets)?;
    ctx.accounts
        .transfer_reserve_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_new_stable(shares)?;
    Ok(())
}

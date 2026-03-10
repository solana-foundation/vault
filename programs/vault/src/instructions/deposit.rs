use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{self, mint_to, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    error::VaultProgramError,
    state::{
        deposit_hook, Rounding, VaultConfig, DEPOSIT_ACCOUNT_METAS_SEED, EXTRA_ACCOUNT_METAS_SEED,
        MAX_BPS, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
    },
};

#[derive(Accounts)]
pub struct DepositAndMint<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    // Not checking the mint authority is expected behaviour
    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = vault,
        token::token_program = asset_token_program,
        seeds = [RESERVE_CONFIG_SEED, share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    #[account(
        mut,
        token::authority = vault.fee_recipient,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub fee_recipient: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
    )]
    pub user_assets_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = share_mint,
        token::token_program = share_token_program,
    )]
    pub user_shares_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [EXTRA_ACCOUNT_METAS_SEED,DEPOSIT_ACCOUNT_METAS_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub extra_metas: Option<AccountInfo<'info>>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    /// CHECK: This is the hook program ID
    pub hook_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositAndMint<'info> {
    pub fn transfer_asset_token_fee_to_fee_recipient(&mut self, fee: u64) -> Result<()> {
        let fee_recipient_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.fee_recipient.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            self.asset_token_program.to_account_info(),
            fee_recipient_transfer_cpi_accounts,
        );

        token_interface::transfer_checked(cpi_ctx, fee, self.asset_mint.decimals)
    }

    pub fn transfer_asset_token_to_vault(&mut self, amount: u64) -> Result<()> {
        let vault_transfer_cpi_accounts = TransferChecked {
            from: self.user_assets_account.to_account_info(),
            mint: self.asset_mint.to_account_info(),
            to: self.reserve.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.asset_token_program.to_account_info(),
            vault_transfer_cpi_accounts,
        );
        token_interface::transfer_checked(cpi_ctx, amount, self.asset_mint.decimals)
    }

    pub fn mint_shares_to_user(&mut self, amount: u64) -> Result<()> {
        let share_mint = self.share_mint.key();
        let mint_to_cpi_accounts = MintTo {
            mint: self.share_mint.to_account_info(),
            to: self.user_shares_account.to_account_info(),
            authority: self.vault.to_account_info(),
        };

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];

        let mint_cpi_ctx = CpiContext::new_with_signer(
            self.share_token_program.to_account_info(),
            mint_to_cpi_accounts,
            seeds,
        );
        mint_to(mint_cpi_ctx, amount)
    }

    pub fn get_transfer_fees(&mut self, amount: u64) -> Result<u64> {
        if self.asset_mint.to_account_info().owner == &spl_token::id() {
            return Ok(0);
        }
        let binding = self.asset_mint.to_account_info();
        let mint_data = binding.data.borrow();
        let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
        let transfer_fee_config =
            mint.get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>()?;
        let transfer_fee: u16 = transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points
            .into();
        Ok(amount
            .checked_mul(transfer_fee.into())
            .ok_or(VaultProgramError::ArithmeticError)?
            .div_ceil(MAX_BPS.into()))
    }

    pub fn deposit_hook(&mut self) -> Result<()> {
        let extra_metas = &self.extra_metas.clone().unwrap();
        let share_mint = self.share_mint.key();
        let deposit_hook_ix = deposit_hook(
            &self.hook_program.key(),
            &self.vault.key(),
            &self.share_mint.key(),
            &extra_metas.key(),
            &self.system_program.key(),
        );

        let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint.as_ref(), &[self.vault.bump]]];
        let account_infos = vec![
            self.vault.to_account_info(),
            self.share_mint.to_account_info(),
            extra_metas.to_account_info(),
            self.system_program.to_account_info(),
        ];

        invoke_signed(&deposit_hook_ix, &account_infos, seeds)?;
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<DepositAndMint>, assets: u64, min_shares: u64) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;

    let fee = ctx.accounts.vault.get_deposit_fee(assets)?;
    // current vault amount
    let reserve_amount_before = ctx.accounts.reserve.amount;
    // transfer assets in case there are transfer fees (Token2022)
    let remaining_amount = assets
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    ctx.accounts
        .transfer_asset_token_to_vault(remaining_amount)?;
    ctx.accounts.reserve.reload()?;

    let is_deposit_hook_present = ctx.accounts.vault.deposit_hook_type().is_some();

    if is_deposit_hook_present {
        // Delegate
        ctx.accounts.deposit_hook()?;
        // Remove delegation
    }

    let updated_reserve_amount = ctx.accounts.reserve.amount;

    let actual_transferred_amount = updated_reserve_amount
        .checked_sub(reserve_amount_before)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        updated_reserve_amount <= ctx.accounts.vault.vault_asset_cap,
        VaultProgramError::MaxVaultAssetCapExceeded
    );

    let shares = ctx.accounts.vault.get_shares_from_assets(
        reserve_amount_before,
        ctx.accounts.share_mint.supply,
        actual_transferred_amount,
        Rounding::Down,
    )?;

    if shares == 0 {
        return Err(VaultProgramError::InsufficientDepositAmount.into());
    }

    if shares < min_shares {
        return Err(VaultProgramError::SlippageExceeded.into());
    }

    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

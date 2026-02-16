use anchor_lang::prelude::*;

use crate::{error::VaultProgramError, instructions::DepositAndMint, state::Rounding};

pub fn handler<'info>(ctx: Context<DepositAndMint>, shares: u64) -> Result<()> {
    require!(!ctx.accounts.vault.paused, VaultProgramError::PausedVault);

    let assets = ctx.accounts.vault.get_assets_from_shares(
        ctx.accounts.share_mint.supply,
        shares,
        Rounding::Up,
    )?;
    let expected_total_assets = ctx
        .accounts
        .vault
        .total_asset_balance
        .checked_add(assets)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        expected_total_assets <= ctx.accounts.vault.vault_asset_cap,
        VaultProgramError::MaxVaultAssetCapExceeded
    );
    // current vault amount
    let reserve_amount_before = ctx.accounts.reserve.amount;
    ctx.accounts.transfer_asset_token_to_vault(assets)?;
    ctx.accounts.reserve.reload()?;
    let updated_reserve_amount = ctx.accounts.reserve.amount;

    let actual_transferred_amount = updated_reserve_amount
        .checked_sub(reserve_amount_before)
        .ok_or(VaultProgramError::ArithmeticError)?;
    let fee = ctx
        .accounts
        .vault
        .get_deposit_fee_when_minting(actual_transferred_amount)?;
    ctx.accounts
        .vault
        .increase_asset_supply(actual_transferred_amount)?;

    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

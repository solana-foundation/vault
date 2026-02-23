use anchor_lang::prelude::*;

use crate::{error::VaultProgramError, instructions::DepositAndMint, state::Rounding};

pub fn handler<'info>(ctx: Context<DepositAndMint>, shares: u64, max_assets: u64) -> Result<()> {
    require!(!ctx.accounts.vault.paused, VaultProgramError::PausedVault);

    let assets = ctx.accounts.vault.get_assets_from_shares(
        ctx.accounts.reserve.amount,
        ctx.accounts.share_mint.supply,
        shares,
        Rounding::Up,
    )?;

    if assets > max_assets {
        return Err(VaultProgramError::SlippageExceeded.into());
    }

    let transfer_fee: u64 = ctx.accounts.get_transfer_fees(assets)?;
    let assets_plus_transfer_fee = assets
        .checked_add(transfer_fee)
        .ok_or(VaultProgramError::ArithmeticError)?;

    ctx.accounts
        .transfer_asset_token_to_vault(assets_plus_transfer_fee)?;

    require!(
        assets <= ctx.accounts.vault.vault_asset_cap,
        VaultProgramError::MaxVaultAssetCapExceeded
    );
    let fee = ctx.accounts.vault.get_deposit_fee_when_minting(assets)?;
    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

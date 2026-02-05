use anchor_lang::prelude::*;

use crate::{error::VaultProgramError, instructions::Deposit, state::Rounding};

pub fn handler<'info>(ctx: Context<Deposit>, shares: u64) -> Result<()> {
    require!(!ctx.accounts.vault.paused, VaultProgramError::PausedVault);

    let assets = ctx.accounts.vault.get_assets_from_shares(
        ctx.accounts.share_mint.supply,
        shares,
        Rounding::Up,
    )?;
    let fee = ctx.accounts.vault.get_deposit_fee_when_minting(assets)?;
    ctx.accounts.vault.increase_asset_supply(assets)?;
    ctx.accounts.transfer_asset_token_to_vault(assets)?;
    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

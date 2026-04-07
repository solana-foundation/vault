use anchor_lang::prelude::*;

use crate::{error::VaultProgramError, instructions::DepositAndMint, state::Rounding};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, DepositAndMint<'info>>,
    shares: u64,
    max_assets: u64,
) -> Result<()> {
    ctx.accounts.vault.assert_unpaused_and_initialized()?;
    let deposit_hook_program = ctx.accounts.vault.deposit_hook_type();
    let mut nav = ctx.accounts.reserve.amount;
    if deposit_hook_program.is_some() {
        let hook_program_pubkey = ctx.accounts.get_program_hook_pubkey(deposit_hook_program)?;
        nav = ctx
            .accounts
            .get_nav(hook_program_pubkey, ctx.remaining_accounts)?;
    }

    let assets = ctx.accounts.vault.get_assets_from_shares(
        nav,
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
    let deposit_hook_program = ctx.accounts.vault.deposit_hook_type();

    if deposit_hook_program.is_some() {
        let hook_program_pubkey = ctx.accounts.get_program_hook_pubkey(deposit_hook_program)?;

        let remaining = ctx.remaining_accounts;
        ctx.accounts.delegate_reserve(
            ctx.accounts
                .hook_program
                .as_ref()
                .ok_or(VaultProgramError::OptionalAccountIsEmpty)?
                .clone(),
            assets,
        )?;
        ctx.accounts
            .execute_deposit_hook(hook_program_pubkey, remaining, assets)?;
    }
    let fee = ctx.accounts.vault.get_deposit_fee_when_minting(assets)?;
    ctx.accounts
        .transfer_asset_token_fee_to_fee_recipient(fee)?;
    ctx.accounts.mint_shares_to_user(shares)?;
    Ok(())
}

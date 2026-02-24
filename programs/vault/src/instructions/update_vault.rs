use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    state::{FeeType, VaultConfig, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateVaultArgs {
    // "new_authority is intentionally not a signer as this makes it operationally difficult when
    // attempting to set the authority to a PDA or governance multisig."
    new_authority: Option<Pubkey>,
    deposit_fees: Option<FeeType>,
    withdraw_fees: Option<FeeType>,
    vault_asset_cap: Option<u64>,
    paused: Option<bool>,
}

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,
}

pub fn handler<'info>(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
    if let Some(fee) = args.deposit_fees {
        fee.validate()?;
    }
    if let Some(fee) = args.withdraw_fees {
        fee.validate()?;
    }

    let current_deposit_fee = ctx.accounts.vault.deposit_fees;
    let current_withdraw_fee = ctx.accounts.vault.withdraw_fees;
    let current_vault_asset_cap = ctx.accounts.vault.vault_asset_cap;
    let current_paused_status = ctx.accounts.vault.paused;
    let new_authority = args
        .new_authority
        .unwrap_or(ctx.accounts.authority.key())
        .key();

    ctx.accounts.vault.deposit_fees = args.deposit_fees.unwrap_or(current_deposit_fee);
    ctx.accounts.vault.withdraw_fees = args.withdraw_fees.unwrap_or(current_withdraw_fee);
    ctx.accounts.vault.vault_asset_cap = args.vault_asset_cap.unwrap_or(current_vault_asset_cap);
    ctx.accounts.vault.paused = args.paused.unwrap_or(current_paused_status);
    ctx.accounts.vault.authority = new_authority;

    Ok(())
}

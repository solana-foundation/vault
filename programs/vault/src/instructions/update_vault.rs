use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::state::{FeeType, VaultConfig, VAULT_CONFIG_SEED};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateVaultArgs {
    deposit_fees: Option<FeeType>,
    withdraw_fees: Option<FeeType>,
    vault_asset_cap: Option<u64>,
    paused: Option<bool>,
}

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account()]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
    ctx.accounts.vault.set_inner(VaultConfig {
        asset_mint_address: ctx.accounts.asset_mint.key(),
        share_mint_address: ctx.accounts.share_mint.key(),
        vault_token_account: ctx.accounts.reserve.key(),
        authority: ctx.accounts.authority.key(),
        initial_price: 0,
        deposit_fees: args.deposit_fees.unwrap_or(ctx.accounts.vault.deposit_fees),
        withdraw_fees: args
            .withdraw_fees
            .unwrap_or(ctx.accounts.vault.withdraw_fees),
        paused: args.paused.unwrap_or(ctx.accounts.vault.paused),
        vault_asset_cap: args.vault_asset_cap.unwrap_or(0),
        total_asset_balance: 0,
    });
    Ok(())
}

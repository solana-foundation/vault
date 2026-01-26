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
    Ok(())
}

use anchor_lang::{prelude::*};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};


use crate::state::{
         FeeType, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED, VaultConfig
    };


#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateVaultArgs {
    deposit_fees: Option<FeeType>,
    withdraw_fees: Option<FeeType>,
    vault_asset_cap: Option<u64>
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
        init,
        token::mint = asset_mint, 
        token::authority = reserve,
        payer = payer,
        seeds = [RESERVE_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        space = 8 + VaultConfig::INIT_SPACE,
        payer = payer,
        seeds = [VAULT_CONFIG_SEED, asset_mint.key().as_ref(), share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(
    ctx: Context<UpdateVault>,
    args: UpdateVaultArgs
) -> Result<()> {
    
    
    Ok(())
}
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::state::{VaultConfig, VAULT_CONFIG_SEED};

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        space = 8 + VaultConfig::INIT_SPACE,
        payer = payer,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<CreateVault>) -> Result<()> {
    ctx.accounts.vault.set_inner(VaultConfig {
        amount_deposit: 0,
        bump: ctx.bumps.vault,
    });
    Ok(())
}

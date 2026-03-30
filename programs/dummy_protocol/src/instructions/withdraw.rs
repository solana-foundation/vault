use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::DummyProgramError,
    state::{VaultConfig, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<Withdraw>, assets: u64) -> Result<()> {
    ctx.accounts.vault.amount_deposit = ctx
        .accounts
        .vault
        .amount_deposit
        .checked_sub(assets)
        .ok_or(DummyProgramError::ArithmeticError)?;
    Ok(())
}

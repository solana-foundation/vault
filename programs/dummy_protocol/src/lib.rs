use anchor_lang::prelude::*;
pub mod error;
pub mod instructions;
pub mod state;

declare_id!("BTNuRUYMNxqg9XfndGm2DiSjrc14QLfNt7BbhMnLZaV");

use instructions::*;

#[program]
pub mod dummy_protocol {
    use super::*;

    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
        instructions::create_vault::handler(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
        instructions::deposit::handler(ctx, assets)
    }

    pub fn withdraw(ctx: Context<Withdraw>, assets: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, assets)
    }
}

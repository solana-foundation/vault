use anchor_lang::prelude::*;
pub mod error;
pub mod instructions;
pub mod state;

declare_id!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw");

use instructions::*;

#[program]
pub mod vault {

    use super::*;

    /// Initialize an atomic vault.
    pub fn create_vault(ctx: Context<CreateVault>, args: VaultArgs) -> Result<()> {
        instructions::create_vault::handler(ctx, args)
    }

    /// Closes an atomic vault.
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::close_vault::handler(ctx)
    }

    /// Update a tokenized vault state based on the supplied arguments.
    pub fn update_vault(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
        instructions::update_vault::handler(ctx, args)
    }

    /// Deposit to the atomic vault.
    pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
        instructions::deposit::handler(ctx, assets)
    }

    /// Mint shares from the atomic vault.
    pub fn mint(ctx: Context<Deposit>, shares: u64) -> Result<()> {
        instructions::mint::handler(ctx, shares)
    }
}

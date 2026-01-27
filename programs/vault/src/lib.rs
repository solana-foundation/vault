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
    /// Update a tokenized vault state based on the supplied arguments.
    pub fn update_vault(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
        instructions::update_vault::handler(ctx, args)
    }
}

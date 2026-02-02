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
    /// Donate assets into the vault.
    /// Transfers the specified amount of asset tokens to the vault's reserve account.
    ///
    /// # Arguments
    /// * `assets` - The amount of asset tokens to deposit into the vault
    pub fn donate_assets(ctx: Context<DonateAssets>, assets: u64) -> Result<()> {
        instructions::donate_assets::handler(ctx, assets)
    }
}

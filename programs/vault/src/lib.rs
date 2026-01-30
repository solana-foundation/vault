use anchor_lang::prelude::*;
pub mod error;
pub mod instructions;
pub mod state;

declare_id!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw");

use instructions::*;

#[program]
pub mod vault {

    use super::*;

    /// Initialize a new tokenized vault with configurable fees and asset cap.
    /// Creates a vault config account, share token mint, and reserve token account.
    /// It also sets the vault as the mint authority
    ///
    /// # Arguments
    /// * `authority` - The pubkey that will control vault operations and updates
    /// * `initial_price` - The starting conversion rate between assets and shares
    /// * `deposit_fees` - Optional fee configuration applied when users deposit assets
    /// * `withdraw_fees` - Optional fee configuration applied when users withdraw assets
    /// * `vault_asset_cap` - Optional maximum amount of assets the vault can hold
    /// * `fee_recipient` - The pubkey that will receive collected fees
    pub fn create_vault(ctx: Context<CreateVault>, args: VaultArgs) -> Result<()> {
        instructions::create_vault::handler(ctx, args)
    }

    /// Closes a vault and transfers all remaining assets to a specified destination.
    /// Transfers any remaining reserve assets to the provided destination token account,
    /// closes the reserve token account, and closes the vault config account.
    /// Only the vault authority can close the vault.
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::close_vault::handler(ctx)
    }

    /// Updates vault configuration parameters.
    /// Allows modifying the authority, deposit/withdraw fees, asset cap, and paused state.
    /// Only the current vault authority can perform updates.
    ///
    /// # Arguments
    /// * `new_authority` - Optional new authority pubkey (can be PDA or multisig, doesn't need to sign)
    /// * `deposit_fees` - Optional updated fee configuration for deposits
    /// * `withdraw_fees` - Optional updated fee configuration for withdrawals
    /// * `vault_asset_cap` - Optional updated maximum asset capacity
    /// * `paused` - Optional flag to pause/unpause vault operations
    pub fn update_vault(ctx: Context<UpdateVault>, args: UpdateVaultArgs) -> Result<()> {
        instructions::update_vault::handler(ctx, args)
    }

    /// Deposits assets into the vault and mints shares to the depositor ATA.
    /// Transfers the specified amount of asset tokens to the vault's reserve account
    /// and mints corresponding share tokens based on the vault's current price.
    /// Applies deposit fees if configured.
    ///
    /// # Arguments
    /// * `assets` - The amount of asset tokens to deposit into the vault
    pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
        instructions::deposit::handler(ctx, assets)
    }
}

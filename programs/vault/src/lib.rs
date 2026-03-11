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
    /// Creates the vault config account and reserve token account.
    /// Sets the vault as mint authority for the provided share mint.
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

    /// Closes a vault after reserves are emptied and share supply is zero.
    /// Closes the reserve token account and the vault config account.
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::close_vault::handler(ctx)
    }

    /// Updates vault configuration parameters.
    /// Allows modifying the authority, deposit/withdraw fees, asset cap, and paused state.
    /// Only the current vault authority can perform updates.
    ///
    /// # Arguments
    /// * `new_authority` - Optional new authority pubkey (can be PDA or multisig, doesn't need to
    ///   sign)
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
    /// * `min_shares` - Minimum number of shares the user must receive (slippage check)
    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositAndMint<'info>>,
        assets: u64,
        min_shares: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, assets, min_shares)
    }

    /// Mint shares from the atomic vault.
    /// # Arguments
    /// * `shares` - The amount of shares to mint to the user
    /// * `max_assets` - Maximum amount of asset tokens the user is willing to pay (slippage check)
    pub fn mint(ctx: Context<DepositAndMint>, shares: u64, max_assets: u64) -> Result<()> {
        instructions::mint::handler(ctx, shares, max_assets)
    }

    /// Withdraws assets from the vault by burning the required amount of shares.
    /// Burns shares from the user's shares Token account and transfers the requested amount of
    /// asset tokens from the vault's reserve account to the user's assets ATA.
    /// The number of shares to burn is computed using the vault's current price and
    /// rounded up to ensure the user burns enough shares to cover the withdrawal.
    ///
    /// # Arguments
    /// * `assets` - The amount of asset tokens to withdraw from the vault
    /// * `max_shares` - Maximum number of shares the user is willing to burn (slippage check)
    pub fn withdraw(ctx: Context<Withdraw>, assets: u64, max_shares: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, assets, max_shares)
    }

    /// Redeems shares for assets.
    /// Burns `shares` from the user's shares Token account and transfers the corresponding amount
    /// of asset tokens from the vault's reserve account to the user's assets ATA.
    /// Fees, if any, are taken from the total assets.
    ///
    /// # Arguments
    /// * `shares` - The amount of shares to redeem for asset tokens
    /// * `min_assets` - Minimum amount of asset tokens the user must receive (slippage check)
    pub fn redeem(ctx: Context<Redeem>, shares: u64, min_assets: u64) -> Result<()> {
        instructions::redeem::handler(ctx, shares, min_assets)
    }

    /// Marks the vault as initialized (locks further pre-init configuration).
    /// Once initialized, fee extension initialization is no longer allowed.
    /// Only the vault authority can call this.
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault::handler(ctx)
    }

    /// Initializes the deposit fee extension for a vault (one-time, pre-init only).
    /// Stores the provided fee config inside `vault.extensions` as `VaultExtension::DepositFee`.
    /// Only the vault authority can call this.
    ///
    /// # Arguments
    /// * `deposit_fee` - The fee configuration to apply on deposits
    pub fn initialize_deposit_fees(
        ctx: Context<InitDepositFees>,
        args: InitDepositFeesArgs,
    ) -> Result<()> {
        instructions::initialize_deposit_fees::handler(ctx, args)
    }

    /// Initializes the withdrawal fee extension for a vault (one-time, pre-init only).
    /// Stores the provided fee config inside `vault.extensions` as `VaultExtension::WithdrawalFee`.
    /// Only the vault authority can call this.
    ///
    /// # Arguments
    /// * `withdrawal_fee` - The fee configuration to apply on withdrawals
    pub fn initialize_withdrawal_fees(
        ctx: Context<InitWithdrawalFees>,
        args: InitWithdrawalFeesArgs,
    ) -> Result<()> {
        instructions::initialize_withdrawal_fees::handler(ctx, args)
    }

    /// Initializes the deposit hook extension for a vault (one-time, pre-init only).
    /// Stores the provided extension inside `vault.extensions` as `VaultExtension::DepositHook`.
    /// Only the vault authority can call this.
    pub fn initialize_deposit_hook(ctx: Context<InitializeDepositHook>) -> Result<()> {
        instructions::initialize_deposit_hook_extension::handler(ctx)
    }

    /// Updates the deposit fee configuration for an already-initialized deposit fee extension.
    /// Finds the existing `VaultExtension::DepositFee` entry and replaces it in-place.
    /// Only the vault authority can call this.
    ///
    /// # Arguments
    /// * `new_deposit_fee` - The new fee configuration to apply on deposits
    pub fn update_deposit_fees(
        ctx: Context<UpdateDepositFees>,
        args: UpdateDepositFeesArgs,
    ) -> Result<()> {
        instructions::update_deposit_fees::handler(ctx, args)
    }

    /// Updates the withdrawal fee configuration for an already-initialized withdrawal fee
    /// extension. Finds the existing `VaultExtension::WithdrawalFee` entry and replaces it
    /// in-place. Only the vault authority can call this.
    ///
    /// # Arguments
    /// * `new_withdrawal_fee` - The new fee configuration to apply on withdrawals
    pub fn update_withdrawal_fees(
        ctx: Context<UpdateWithdrawalFees>,
        args: UpdateWithdrawalFeesArgs,
    ) -> Result<()> {
        instructions::update_withdrawal_fees::handler(ctx, args)
    }

    // Extra Meta Accounts

    /// Initializes the deposit hook extra meta accounts needed for the deposit hook
    pub fn initialize_deposit_extra_meta_accounts(
        ctx: Context<InitializeDepositExtraMetaAccounts>,
    ) -> Result<()> {
        instructions::initialize_deposit_extra_meta_accounts::handler(ctx)
    }
}

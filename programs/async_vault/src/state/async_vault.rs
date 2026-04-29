use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    utils::{calculate_assets, calculate_shares},
};

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub asset_mint: Pubkey,
    /// share mint address
    pub share_mint: Pubkey,
    /// token account holding confirmed vault assets
    pub vault_token_account: Pubkey,
    /// authority that can sign permissioned instructions
    pub authority: Pubkey,
    /// pubkey that is required to own the TokenAccount fees are sent to
    pub fee_recipient: Pubkey,
    /// initial price of shares in asset units (scaled by asset mint decimals)
    pub initial_price: u64,
    /// paused
    pub paused: bool,
    /// once a vault is initialized, no extensions can be added
    pub initialized: bool,
    /// token account holding assets from deposits awaiting share issuance
    pub pending_vault: Pubkey,
    /// net asset value (assets per share), default 0 until first NAV update
    pub nav: u128,
    /// nav version, incremented on each NAV update
    pub nav_version: u64,
    /// whether deposits are processed asynchronously
    pub async_inflows: bool,
    /// whether withdrawals are processed asynchronously
    pub async_outflows: bool,
    /// count of pending async deposit/withdrawal requests
    pub pending_async_requests: u16,
    /// virtual vault asset balance, accounts for tokens that may
    /// have been withdrawn by the vault authority
    pub total_asset_balance: u64,
    pub reserve_bump: u8,
    pub pending_vault_bump: u8,
    pub bump: u8,
    // Used for updating the vault authority (New Authority)
    pub pending_authority: Option<Pubkey>,
}

impl Vault {
    pub fn assert_unpaused_and_initialized(&self) -> Result<()> {
        require!(self.initialized, AsyncVaultError::UninitializedVault);
        require!(!self.paused, AsyncVaultError::PausedVault);
        Ok(())
    }

    pub fn assert_uninitialized(&self) -> Result<()> {
        require!(!self.initialized, AsyncVaultError::VaultAlreadyInitialized);
        Ok(())
    }

    /// Converts an asset amount into shares using the current NAV.
    ///
    /// `shares = net_amount * 10^decimals / nav`
    pub fn calculate_shares(&mut self, decimals: u8, net_amount: u64) -> Result<u64> {
        calculate_shares(self.nav, decimals, net_amount)
    }

    /// Converts a share amount into assets using the current NAV.
    ///
    /// `assets = share_amount * nav / 10^decimals`
    pub fn calculate_assets(&self, decimals: u8, share_amount: u64) -> Result<u64> {
        calculate_assets(self.nav, decimals, share_amount)
    }
}

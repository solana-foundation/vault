use anchor_lang::prelude::*;

use crate::extensions::VaultExtension;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub asset_mint_address: Pubkey,
    /// share mint address
    pub share_mint_address: Pubkey,
    /// token account holding confirmed vault assets
    pub vault_token_account: Pubkey,
    /// authority that can sign permissioned instructions
    pub authority: Pubkey,
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
    #[max_len(5)]
    pub extensions: Vec<VaultExtension>,
}

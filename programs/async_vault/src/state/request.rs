use anchor_lang::prelude::*;

use crate::utils::{calculate_assets, calculate_shares};

/// Pending: neither approved nor rejected by the vault authority
/// Claimable: approved by the vault authority
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq)]
pub enum RequestState {
    Pending,
    Claimable,
    Rejected,
}

/// The request types:
/// Deposit: the user wants to add assets to the vault
/// Redeem: the user wants to withdraw assets from the vault
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq)]
pub enum RequestType {
    Deposit,
    Redeem,
}

/// Request account state for tracking an async deposit/redemption.
#[account]
#[derive(InitSpace)]
pub struct Request {
    /// Vault address
    pub vault: Pubkey,
    /// request type
    pub request_type: RequestType,
    /// request state
    pub request_state: RequestState,
    /// User that made the request
    pub owner: Pubkey,
    /// RequestType::Deposit - amount of assets being deposited
    /// RequestType::Redeem - amount of shares being redeemed
    pub amount: u64,
    /// NAV at which the assets (deposit) or shares (redeem) are being converted
    pub price: u128,
    /// mint address for deposit request (7575)
    pub asset_mint_address: Pubkey,
    /// timestamp, slot or epoch
    pub created_at: i64,
    /// nav update version (for permissionless actions)
    pub nav_update_version: u64,
    /// Operator allowed to claim on behalf of user (delegated controller)
    pub operator: Option<Pubkey>,
}

impl Request {
    /// Converts an asset amount into shares using the current NAV.
    ///
    /// `shares = net_amount * 10^decimals / price`
    pub fn calculate_shares(&mut self, decimals: u8, net_amount: u64) -> Result<u64> {
        calculate_shares(self.price, decimals, net_amount)
    }

    /// Converts a share amount into assets using the current NAV.
    ///
    /// `assets = share_amount * price / 10^decimals`
    pub fn calculate_assets(&self, decimals: u8, share_amount: u64) -> Result<u64> {
        calculate_assets(self.price, decimals, share_amount)
    }
}

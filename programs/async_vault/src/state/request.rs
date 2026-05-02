use anchor_lang::prelude::*;

/// Pending: neither approved nor rejected by the vault authority
/// Claimable: approved by the vault authority
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq)]
pub enum RequestState {
    Pending,
    Claimable,
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

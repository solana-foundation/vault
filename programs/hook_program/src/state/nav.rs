use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultAssociatedProtocols {
    #[max_len(10)]
    pub protocols: Vec<Pubkey>,
    pub vault: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ProtocolDeposits {
    pub vault: Pubkey,
    pub protocol: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct NavReturnData {
    /// Current Net Asset Value in vault base units
    pub nav: u64,
    /// Unix timestamp of last NAV update
    pub update_timestamp: i64,
}

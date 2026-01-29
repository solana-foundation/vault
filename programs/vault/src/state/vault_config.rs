use anchor_lang::prelude::*;

use crate::error::VaultProgramError;

/// The fee types:
/// FixedAmount: a fixed fee is applied (ex 0.1 asset)
/// Percentage: the fee is a % of the transfer amount
#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum FeeType {
    NoFee,
    FixedAmount { amount: u64 },
    Percentage { bps: u16 },
}

impl FeeType {
    pub fn validate(self) -> Result<()> {
        match self {
            FeeType::Percentage { bps } => {
                require!(bps <= 10_000, VaultProgramError::FeeBPSLimitReached);
            }
            FeeType::NoFee | FeeType::FixedAmount { .. } => {}
        }
        Ok(())
    }
}

/// Core state of the Vault account necessary for common
/// logic across configuration types.
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub asset_mint_address: Pubkey,
    /// share mint address
    pub share_mint_address: Pubkey,
    /// vault_token_account
    pub vault_token_account: Pubkey,
    /// authority that can sign permissioned instructions
    pub authority: Pubkey,
    /// initial price of shares in asset units (scaled by asset mint decimals)
    pub initial_price: u64,
    /// deposit fees
    pub deposit_fees: FeeType,
    /// withdraw fees
    pub withdraw_fees: FeeType,
    /// paused
    pub paused: bool,
    /// max balance allowed in vault
    pub vault_asset_cap: u64,
    /// virtual vault asset balance
    pub total_asset_balance: u64,
    pub reserve_bump: u8,
    pub bump: u8,
}

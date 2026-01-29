use crate::{error::VaultProgramError, state::Rounding};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

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
    pub fn get_fee(self, total_amount: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                let fee = total_amount
                    .checked_mul(bps.into())
                    .ok_or(VaultProgramError::ArithmeticError)?
                    .checked_add(9_999)
                    .ok_or(VaultProgramError::ArithmeticError)?
                    .checked_div(10_000)
                    .ok_or(VaultProgramError::ArithmeticError)?;
                return Ok(fee);
            }
            FeeType::FixedAmount { amount } => return Ok(amount),
            FeeType::NoFee => return Ok(0),
        }
    }
}

/// Core state of the Vault account necessary for common
/// logic across configuration types.
#[account]
#[derive(InitSpace, Copy)]
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
    /// pubkey that is required to own the TokenAccount fees are sent to
    pub fee_recipient: Pubkey,
    pub reserve_bump: u8,
    pub bump: u8,
}

impl VaultConfig {
    pub fn total_assets(self) -> u64 {
        return self.total_asset_balance;
    }
    pub fn get_shares_from_assets(
        self,
        share_mint: &InterfaceAccount<'_, Mint>,
        share_amount: u64,
        rounding: Rounding,
    ) -> Result<u64> {
        let assets_times_total_supply = u128::from(
            share_mint
                .supply
                .checked_add(1)
                .ok_or(VaultProgramError::ArithmeticError)?,
        )
        .checked_mul(u128::from(share_amount))
        .ok_or(VaultProgramError::ArithmeticError)?;
        let result = match rounding {
            Rounding::Up => assets_times_total_supply.div_ceil(u128::from(
                self.total_assets()
                    .checked_add(1)
                    .ok_or(VaultProgramError::ArithmeticError)?,
            )),
            Rounding::Down => assets_times_total_supply
                .checked_div(u128::from(self.total_assets() + 1))
                .ok_or(VaultProgramError::ArithmeticError)?,
        };
        u64::try_from(result).or(Err(VaultProgramError::ArithmeticError.into()))
    }

    pub fn increase_asset_supply(&mut self, amount: u64) -> Result<()> {
        let new_supply = self
            .total_asset_balance
            .checked_add(amount)
            .ok_or(VaultProgramError::ArithmeticError)?;
        self.total_asset_balance = new_supply;
        Ok(())
    }

    pub fn get_deposit_fee(self, deposit_amount: u64) -> Result<u64> {
        self.deposit_fees.get_fee(deposit_amount)
    }
}

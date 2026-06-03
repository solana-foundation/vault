use anchor_lang::prelude::{borsh, AnchorDeserialize, AnchorSerialize, InitSpace};

use crate::{constants::MAX_BPS, error::VaultMathError};

/// The fee types:
/// FixedAmount: a fixed fee is applied (ex 0.1 asset)
/// Percentage: the fee is a % of the transfer amount
#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum FeeType {
    FixedAmount { amount: u64 },
    Percentage { bps: u16 },
}

impl FeeType {
    /// Ensures percentage fees don't exceed MAX_BPS.
    pub fn validate(self) -> Result<(), VaultMathError> {
        if let FeeType::Percentage { bps } = self {
            if bps > MAX_BPS {
                return Err(VaultMathError::FeeBpsLimitReached);
            }
        }
        Ok(())
    }

    /// Computes the fee from a known total amount (fee-inclusive).
    /// Rounds up for percentage fees.
    pub fn get_fee(self, total_amount: u64) -> Result<u64, VaultMathError> {
        match self {
            FeeType::Percentage { bps } => {
                let fee = total_amount
                    .checked_mul(bps.into())
                    .ok_or(VaultMathError::ArithmeticError)?
                    .checked_add(9_999)
                    .ok_or(VaultMathError::ArithmeticError)?
                    .checked_div(10_000)
                    .ok_or(VaultMathError::ArithmeticError)?;
                Ok(fee)
            }
            FeeType::FixedAmount { amount } => Ok(amount),
        }
    }

    /// Back-derives the fee from the gross (fee-inclusive) withdrawal amount so the user receives
    /// net after fee.
    pub fn get_withdraw_fee_when_redeeming(&self, gross_assets: u64) -> Result<u64, VaultMathError> {
        match self {
            FeeType::Percentage { bps } => {
                if *bps == 0 {
                    return Ok(0);
                }
                // fee = ceil(gross * bps / (MAX_BPS + bps))
                // Derived from: fee = net * bps / MAX_BPS where net = gross - fee
                let denominator = u128::from(MAX_BPS)
                    .checked_add(u128::from(*bps))
                    .ok_or(VaultMathError::ArithmeticError)?;
                let fee = u128::from(gross_assets)
                    .checked_mul(u128::from(*bps))
                    .ok_or(VaultMathError::ArithmeticError)?
                    .div_ceil(denominator);
                u64::try_from(fee).map_err(|_| VaultMathError::ArithmeticError)
            }
            FeeType::FixedAmount { amount } => Ok(*amount),
        }
    }

    /// Computes the deposit fee given the desired net deposit, so gross = net + fee.
    pub fn get_deposit_fee_when_minting(&self, net_assets: u64) -> Result<u64, VaultMathError> {
        match self {
            FeeType::Percentage { bps } => {
                let gross = if *bps == MAX_BPS {
                    net_assets
                        .checked_mul(2)
                        .ok_or(VaultMathError::ArithmeticError)?
                        .into()
                } else {
                    u128::from(net_assets)
                        .checked_mul(MAX_BPS.into())
                        .ok_or(VaultMathError::ArithmeticError)?
                        .checked_div(
                            MAX_BPS
                                .checked_sub(*bps)
                                .ok_or(VaultMathError::ArithmeticError)?
                                .into(),
                        )
                        .ok_or(VaultMathError::ArithmeticError)?
                };

                let fee = if *bps == 0 {
                    0
                } else {
                    gross
                        .checked_sub(u128::from(net_assets))
                        .ok_or(VaultMathError::ArithmeticError)?
                };
                u64::try_from(fee).map_err(|_| VaultMathError::ArithmeticError)
            }
            FeeType::FixedAmount { amount } => Ok(*amount),
        }
    }
}

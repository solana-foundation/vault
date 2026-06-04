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
}

use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::MAX_BPS};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum FeeType {
    FixedAmount { amount: u64 },
    Percentage { bps: u16 },
}

impl FeeType {
    pub fn validate(self) -> Result<()> {
        match self {
            FeeType::Percentage { bps } => {
                require!(bps <= MAX_BPS, AsyncVaultError::FeeBpsExceeded);
            }
            FeeType::FixedAmount { .. } => {}
        }
        Ok(())
    }

    pub fn get_fee(self, total_amount: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                let fee = total_amount
                    .checked_mul(bps.into())
                    .ok_or(AsyncVaultError::ArithmeticError)?
                    .checked_add(9_999)
                    .ok_or(AsyncVaultError::ArithmeticError)?
                    .checked_div(10_000)
                    .ok_or(AsyncVaultError::ArithmeticError)?;
                Ok(fee)
            }
            FeeType::FixedAmount { amount } => Ok(amount),
        }
    }

    pub fn get_deposit_fee_when_minting(&self, net_assets: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                let gross = if *bps == MAX_BPS {
                    net_assets
                        .checked_mul(2)
                        .ok_or(AsyncVaultError::ArithmeticError)?
                        .into()
                } else {
                    u128::from(net_assets)
                        .checked_mul(MAX_BPS.into())
                        .ok_or(AsyncVaultError::ArithmeticError)?
                        .checked_div(
                            MAX_BPS
                                .checked_sub(*bps)
                                .ok_or(AsyncVaultError::ArithmeticError)?
                                .into(),
                        )
                        .ok_or(AsyncVaultError::ArithmeticError)?
                };

                let fee = if *bps == 0 {
                    0
                } else {
                    gross
                        .checked_sub(u128::from(net_assets))
                        .ok_or(AsyncVaultError::ArithmeticError)?
                };
                Ok(u64::try_from(fee)?)
            }
            FeeType::FixedAmount { amount } => Ok(*amount),
        }
    }

    pub fn get_withdraw_fee_when_redeeming(&self, gross_assets: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                if *bps == 0 {
                    return Ok(0);
                }
                let denominator = u128::from(MAX_BPS)
                    .checked_add(u128::from(*bps))
                    .ok_or(AsyncVaultError::ArithmeticError)?;
                let fee = u128::from(gross_assets)
                    .checked_mul(u128::from(*bps))
                    .ok_or(AsyncVaultError::ArithmeticError)?
                    .div_ceil(denominator);
                Ok(u64::try_from(fee)?)
            }
            FeeType::FixedAmount { amount } => Ok(*amount),
        }
    }
}

use crate::{error::VaultProgramError, state::Rounding};
use anchor_lang::prelude::*;

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
        supply: u64,
        asset_amount: u64,
        rounding: Rounding,
    ) -> Result<u64> {
        let assets_times_total_supply: u128 = if supply == 0 {
            u128::from(self.initial_price)
                .checked_mul(u128::from(asset_amount))
                .ok_or(VaultProgramError::ArithmeticError)?
        } else {
            u128::from(
                supply
                    .checked_add(1)
                    .ok_or(VaultProgramError::ArithmeticError)?,
            )
            .checked_mul(u128::from(asset_amount))
            .ok_or(VaultProgramError::ArithmeticError)?
        };
        let result = match rounding {
            Rounding::Up => assets_times_total_supply.div_ceil(u128::from(
                self.total_assets()
                    .checked_add(1)
                    .ok_or(VaultProgramError::ArithmeticError)?,
            )),
            Rounding::Down => assets_times_total_supply
                .checked_div(u128::from(
                    self.total_assets()
                        .checked_add(1)
                        .ok_or(VaultProgramError::ArithmeticError)?,
                ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn create_vault_config(total_asset_balance: u64, initial_price: u64) -> VaultConfig {
        VaultConfig {
            asset_mint_address: Pubkey::new_unique(),
            share_mint_address: Pubkey::new_unique(),
            vault_token_account: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            initial_price,
            deposit_fees: FeeType::NoFee,
            withdraw_fees: FeeType::NoFee,
            paused: false,
            vault_asset_cap: u64::MAX,
            total_asset_balance,
            fee_recipient: Pubkey::new_unique(),
            reserve_bump: 255,
            bump: 254,
        }
    }

    #[test_case(0, 1_000_000, 0, 100, Rounding::Down, 100_000_000;"Initial deposit zero supply zero balance round down")]
    #[test_case(0, 1_000_000, 0, 100, Rounding::Up, 100_000_000;"Initial deposit zero supply zero balance round up")]
    #[test_case(0, 1_000_000, 1000, 100, Rounding::Down, 100_100;"Initial deposit zero supply positive balance round down")]
    #[test_case(5000, 1_000_000, 0, 100, Rounding::Down, 19_996;"Initial deposit zero supply positive balance round down 2")]
    #[test_case(5000, 1_000_000, 0, 100, Rounding::Up, 19_997;"Initial deposit zero supply positive balance round up 2")]
    #[test_case(5000, 1_000_000, 0, 100, Rounding::Down, 19_996;"Initial deposit zero supply positive balance round down 3")]
    #[test_case(10_000, 1_000_000, 10_000, 100, Rounding::Down, 100;"Initial deposit positive supply positive balance round down 2")]
    #[test_case(20_000, 1_000_000, 10_000, 100, Rounding::Down, 50;"Initial deposit positive supply positive balance round down 3")]
    #[test_case(10_000, 1_000_000, 9_999, 100, Rounding::Down, 99;"Initial deposit positive supply positive balance round down 4")]
    #[test_case(10_000, 1_000_000, 9_999, 100, Rounding::Up, 100;"Initial deposit positive supply positive balance round up 1")]
    #[test_case(10_000, 1_000_000, 10_000, 0, Rounding::Down, 0;"Initial deposit positive supply zero balance round down, high initial price")]
    #[test_case(0, 1_000_000, 100, 50, Rounding::Down, 5_050;"Initial deposit div by zero prevention")]
    #[test_case(1_000_000_000, 1_000_000, 1_000_000_000, 1_000_000, Rounding::Down, 1_000_000;"Initial Large values no overflow")]
    #[test_case(1_000_000, 1_000_000, 1_000_000, 1, Rounding::Down, 1;"Precision with small amounts")]
    #[test_case(100, 1_000_000, 1_000_000_000, 10, Rounding::Down, 99_009_901;"Assymmetric supply vs assets")]

    fn test_get_shares_from_assets(
        total_asset_balance: u64,
        initial_price: u64,
        supply: u64,
        asset_amount: u64,
        rounding: Rounding,
        expected_shares: u64,
    ) {
        let vault = create_vault_config(total_asset_balance, initial_price);
        let shares = vault
            .get_shares_from_assets(supply, asset_amount, rounding)
            .unwrap();
        assert_eq!(shares, expected_shares);
    }

    #[test_case(1_000_000, 1_000_000, u64::MAX, 100, Rounding::Down;"ERROR: Initial deposit max supply positive balance round down")]
    #[test_case(u64::MAX, 1_000_000, 1_000_000, 100, Rounding::Down;"ERROR: Initial deposit positive supply max balance round down")]
    #[test_case(0, u64::MAX, 0, u64::MAX, Rounding::Down;"ERROR: Multiplication overflow initial price")]
    #[test_case(1, 1_000_000, u64::MAX-1, u64::MAX, Rounding::Down;"ERROR: Multiplication overflow supply")]
    #[test_case(1, 1_000_000, u64::MAX/2, u64::MAX, Rounding::Down;"ERROR: Result overflows")]
    fn test_get_shares_from_assets_arithmetic_errors(
        total_asset_balance: u64,
        initial_price: u64,
        supply: u64,
        asset_amount: u64,
        rounding: Rounding,
    ) {
        let vault = create_vault_config(total_asset_balance, initial_price);

        let result = vault.get_shares_from_assets(supply, asset_amount, rounding);
        assert!(result.is_err());
    }
    #[test]
    fn test_initial_price_variations() {
        let vault_high = create_vault_config(0, 1_000_000_000);

        let shares = vault_high
            .get_shares_from_assets(0, 1, Rounding::Down)
            .unwrap();
        assert_eq!(shares, 1_000_000_000);

        let vault_low = create_vault_config(0, 1);
        let shares_low = vault_low
            .get_shares_from_assets(0, 1_000_000, Rounding::Down)
            .unwrap();
        assert_eq!(shares_low, 1_000_000);
    }
}

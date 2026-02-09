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

    pub fn get_assets_from_shares(
        self,
        shares_supply: u64,
        share_amount: u64,
        rounding: Rounding,
    ) -> Result<u64> {
        require!(shares_supply > 0, VaultProgramError::InvalidState);

        let numerator = u128::from(share_amount)
            .checked_mul(u128::from(self.total_assets()))
            .ok_or(VaultProgramError::ArithmeticError)?;

        let denominator = u128::from(shares_supply);

        let result = match rounding {
            Rounding::Up => numerator.div_ceil(denominator),
            Rounding::Down => numerator
                .checked_div(denominator)
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

    pub fn decrease_asset_supply(&mut self, amount: u64) -> Result<()> {
        let new_supply = self
            .total_asset_balance
            .checked_sub(amount)
            .ok_or(VaultProgramError::ArithmeticError)?;

        self.total_asset_balance = new_supply;

        Ok(())
    }

    pub fn get_deposit_fee(self, deposit_amount: u64) -> Result<u64> {
        self.deposit_fees.get_fee(deposit_amount)
    }

    pub fn get_withdraw_fee(self, withdraw_amount: u64) -> Result<u64> {
        self.withdraw_fees.get_fee(withdraw_amount)
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

    #[test_case(FeeType::NoFee, 0, 0; "NoFee with zero amount")]
    #[test_case(FeeType::NoFee, 100, 0; "NoFee with small amount")]
    #[test_case(FeeType::NoFee, 1_000_000, 0; "NoFee with large amount")]
    #[test_case(FeeType::NoFee, u64::MAX, 0; "NoFee with max amount")]
    #[test_case(FeeType::FixedAmount { amount: 0 }, 0, 0; "FixedAmount zero fee zero amount")]
    #[test_case(FeeType::FixedAmount { amount: 0 }, 1_000_000, 0; "FixedAmount zero fee large amount")]
    #[test_case(FeeType::FixedAmount { amount: 50 }, 0, 50; "FixedAmount fee with zero amount")]
    #[test_case(FeeType::FixedAmount { amount: 100 }, 1_000, 100; "FixedAmount basic fee")]
    #[test_case(FeeType::FixedAmount { amount: 1_000 }, 100, 1_000; "FixedAmount fee larger than amount")]
    #[test_case(FeeType::FixedAmount { amount: u64::MAX }, u64::MAX, u64::MAX; "FixedAmount max fee max amount")]
    #[test_case(FeeType::Percentage { bps: 0 }, 1_000_000, 0; "Percentage zero bps")]
    #[test_case(FeeType::Percentage { bps: 100 }, 0, 0; "Percentage with zero amount")]
    #[test_case(FeeType::Percentage { bps: 100 }, 1_000_000, 10_000; "Percentage 1% of 1M (100 bps)")]
    #[test_case(FeeType::Percentage { bps: 50 }, 1_000_000, 5_000; "Percentage 0.5% of 1M (50 bps)")]
    #[test_case(FeeType::Percentage { bps: 1 }, 1_000_000, 100; "Percentage 0.01% of 1M (1 bps)")]
    #[test_case(FeeType::Percentage { bps: 10_000 }, 1_000_000, 1_000_000; "Percentage 100% of 1M (10000 bps)")]
    #[test_case(FeeType::Percentage { bps: 100 }, 100, 1; "Percentage rounding up small amount")]
    #[test_case(FeeType::Percentage { bps: 100 }, 99, 1; "Percentage rounding up tiny amount")]
    #[test_case(FeeType::Percentage { bps: 100 }, 1, 1; "Percentage rounding up minimal amount")]
    #[test_case(FeeType::Percentage { bps: 1 }, 10_000, 1; "Percentage 1 bps rounds up")]
    #[test_case(FeeType::Percentage { bps: 50 }, 10_000, 50; "Percentage 50 bps on 10k")]
    #[test_case(FeeType::Percentage { bps: 100 }, 10_000, 100; "Percentage 100 bps on 10k")]
    #[test_case(FeeType::Percentage { bps: 10_000 }, 100, 100; "Percentage 100% fee equals amount")]
    #[test_case(FeeType::Percentage { bps: 5_000 }, 100, 50; "Percentage 50% fee")]
    #[test_case(FeeType::Percentage { bps: 1 }, 100, 1; "Percentage very small bps")]
    #[test_case(FeeType::Percentage { bps: 9_999 }, 10_000, 9_999; "Percentage near 100%")]
    #[test_case(FeeType::Percentage { bps: 100 }, 1_000_000_000, 10_000_000; "Percentage on 1B")]
    #[test_case(FeeType::Percentage { bps: 1 }, 1_000_000_000, 100_000; "Percentage 1 bps on 1B")]

    fn test_get_fee(fee_type: FeeType, total_amount: u64, expected_fee: u64) {
        let fee = fee_type.get_fee(total_amount).unwrap();
        assert_eq!(fee, expected_fee);
    }

    #[test_case(FeeType::Percentage { bps: 10_000 }, u64::MAX;"ERROR: Result overflows on multiply")]
    #[test_case(FeeType::Percentage { bps: u16::MAX  }, u64::MAX;"ERROR: Result overflows on add")]
    fn test_percentage_overflow_on_multiply(fee_type: FeeType, amount: u64) {
        let result = fee_type.get_fee(amount);
        assert!(result.is_err());
    }

    #[test_case(1, 3, 1, Rounding::Down, 0; "ceil differs: 1*1/3 down")]
    #[test_case(1, 3, 1, Rounding::Up, 1; "ceil differs: 1*1/3 up")]
    #[test_case(10_000, 3, 2, Rounding::Down, 6_666; "non-clean division down")]
    #[test_case(10_000, 3, 2, Rounding::Up, 6_667; "non-clean division up")]
    #[test_case(0, 1_000, 100, Rounding::Down, 0; "Zero assets, positive supply, down")]
    #[test_case(0, 1_000, 100, Rounding::Up, 0; "Zero assets, positive supply, up")]
    #[test_case(10_000, 10_000, 100, Rounding::Down, 100; "1:1 ratio down")]
    #[test_case(10_000, 10_000, 100, Rounding::Up, 100; "1:1 ratio up")]
    #[test_case(2_000, 1_000, 100, Rounding::Down, 200; "2:1 assets:supply down")]
    #[test_case(2_000, 1_000, 100, Rounding::Up, 200; "2:1 assets:supply up")]
    #[test_case(9_999, 10_000, 100, Rounding::Down, 99; "Rounding edge down")]
    #[test_case(9_999, 10_000, 100, Rounding::Up, 100; "Rounding edge up")]
    #[test_case(10_000, 10_000, 0, Rounding::Down, 0; "Zero share_amount returns 0")]
    #[test_case(1_000_000_000, 1_000_000_000, 1_000_000, Rounding::Down, 1_000_000; "Large values 1:1")]
    #[test_case(1_000_000, 1_000_000, 1, Rounding::Down, 1; "Precision small amounts")]
    #[test_case(100, 1_000_000_000, 10, Rounding::Down, 0; "Asymmetric small assets/share down")]
    #[test_case(100, 1_000_000_000, 10, Rounding::Up, 1; "Asymmetric small assets/share up")]
    #[test_case(1_000_000_000, 100, 10, Rounding::Down, 100_000_000; "Asymmetric huge assets/share down")]
    #[test_case(1_000_000_000, 100, 10, Rounding::Up, 100_000_000; "Asymmetric huge assets/share up")]
    fn test_get_assets_from_shares(
        total_asset_balance: u64,
        shares_supply: u64,
        share_amount: u64,
        rounding: Rounding,
        expected_assets: u64,
    ) {
        let vault = create_vault_config(total_asset_balance, 1_000_000);

        let assets = vault
            .get_assets_from_shares(shares_supply, share_amount, rounding)
            .unwrap();

        assert_eq!(assets, expected_assets);
    }

    #[test]
    fn get_assets_from_shares_no_share_supply_fails() {
        let vault = create_vault_config(100, 1_000_000);

        let result = vault.get_assets_from_shares(0, 1_000_000, Rounding::Up);

        assert_eq!(result.unwrap_err(), VaultProgramError::InvalidState.into());
    }

    #[test_case(1, u64::MAX, u64::MAX, Rounding::Down; "ERROR: result overflows u64 (down)")]
    #[test_case(1, u64::MAX, u64::MAX, Rounding::Up;   "ERROR: result overflows u64 (up)")]
    fn test_get_assets_from_shares_errors(
        shares_supply: u64,
        total_asset_balance: u64,
        share_amount: u64,
        rounding: Rounding,
    ) {
        let vault = create_vault_config(total_asset_balance, 1_000_000);

        let result = vault.get_assets_from_shares(shares_supply, share_amount, rounding);
        assert_eq!(
            result.unwrap_err(),
            VaultProgramError::ArithmeticError.into()
        );
    }
}

use crate::{
    error::VaultProgramError,
    state::{Rounding, MAX_BPS},
};
use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum VaultExtension {
    DepositFee(FeeType),
    WithdrawalFee(FeeType),
    DepositHook(Pubkey),
}

impl VaultExtension {
    pub fn as_deposit_fee(&self) -> Option<FeeType> {
        match self {
            VaultExtension::DepositFee(fee) => Some(*fee),
            _ => None,
        }
    }

    pub fn as_withdrawal_fee(&self) -> Option<FeeType> {
        match self {
            VaultExtension::WithdrawalFee(fee) => Some(*fee),
            _ => None,
        }
    }

    pub fn as_deposit_hook(&self) -> Option<Pubkey> {
        match self {
            VaultExtension::DepositHook(hook_program) => Some(*hook_program),
            _ => None,
        }
    }
}

/// The fee types:
/// FixedAmount: a fixed fee is applied (ex 0.1 asset)
/// Percentage: the fee is a % of the transfer amount
#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum FeeType {
    FixedAmount { amount: u64 },
    Percentage { bps: u16 },
}

impl FeeType {
    pub fn validate(self) -> Result<()> {
        match self {
            FeeType::Percentage { bps } => {
                require!(bps <= MAX_BPS, VaultProgramError::FeeBPSLimitReached);
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
                    .ok_or(VaultProgramError::ArithmeticError)?
                    .checked_add(9_999)
                    .ok_or(VaultProgramError::ArithmeticError)?
                    .checked_div(10_000)
                    .ok_or(VaultProgramError::ArithmeticError)?;
                return Ok(fee);
            }
            FeeType::FixedAmount { amount } => return Ok(amount),
        }
    }

    pub fn get_withdraw_fee_when_redeeming(&self, gross_assets: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                if *bps == 0 {
                    return Ok(0);
                }
                // fee = ceil(gross * bps / (MAX_BPS + bps))
                // Derived from: fee = net * bps / MAX_BPS where net = gross - fee
                let denominator = u128::from(MAX_BPS)
                    .checked_add(u128::from(*bps))
                    .ok_or(VaultProgramError::ArithmeticError)?;
                let fee = u128::from(gross_assets)
                    .checked_mul(u128::from(*bps))
                    .ok_or(VaultProgramError::ArithmeticError)?
                    .div_ceil(denominator);
                Ok(u64::try_from(fee)?)
            }
            FeeType::FixedAmount { amount } => Ok(*amount),
        }
    }

    pub fn get_deposit_fee_when_minting(&self, net_assets: u64) -> Result<u64> {
        match self {
            FeeType::Percentage { bps } => {
                let gross = if *bps == MAX_BPS {
                    net_assets
                        .checked_mul(2)
                        .ok_or(VaultProgramError::ArithmeticError)?
                        .into()
                } else {
                    u128::from(net_assets)
                        .checked_mul(MAX_BPS.into())
                        .ok_or(VaultProgramError::ArithmeticError)?
                        .checked_div(
                            MAX_BPS
                                .checked_sub(*bps)
                                .ok_or(VaultProgramError::ArithmeticError)?
                                .into(),
                        )
                        .ok_or(VaultProgramError::ArithmeticError)?
                };

                let fee = if *bps == 0 {
                    0
                } else {
                    gross
                        .checked_sub(u128::from(net_assets))
                        .ok_or(VaultProgramError::ArithmeticError)?
                };
                Ok(u64::try_from(fee)?)
            }
            FeeType::FixedAmount { amount } => return Ok(*amount),
        }
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
    /// paused
    pub paused: bool,
    /// once a vault is initialized, no extensions can be added
    pub initialized: bool,
    /// max balance allowed in vault
    pub vault_asset_cap: u64,
    /// pubkey that is required to own the TokenAccount fees are sent to
    pub fee_recipient: Pubkey,
    /// vault extensions
    #[max_len(10)]
    pub extensions: Vec<VaultExtension>,
    pub reserve_bump: u8,
    pub bump: u8,
}

impl VaultConfig {
    pub fn get_shares_from_assets(
        &self,
        reserve_balance: u64,
        share_supply: u64,
        asset_amount: u64,
        rounding: Rounding,
    ) -> Result<u64> {
        let assets_times_total_supply: u128;
        if share_supply == 0 {
            assets_times_total_supply = u128::from(self.initial_price)
                .checked_mul(u128::from(asset_amount))
                .ok_or(VaultProgramError::ArithmeticError)?;
        } else {
            assets_times_total_supply = u128::from(
                share_supply
                    .checked_add(1)
                    .ok_or(VaultProgramError::ArithmeticError)?,
            )
            .checked_mul(u128::from(asset_amount))
            .ok_or(VaultProgramError::ArithmeticError)?;
        }
        let result = match rounding {
            Rounding::Up => assets_times_total_supply.div_ceil(u128::from(
                reserve_balance
                    .checked_add(1)
                    .ok_or(VaultProgramError::ArithmeticError)?,
            )),
            Rounding::Down => assets_times_total_supply
                .checked_div(u128::from(
                    reserve_balance
                        .checked_add(1)
                        .ok_or(VaultProgramError::ArithmeticError)?,
                ))
                .ok_or(VaultProgramError::ArithmeticError)?,
        };
        u64::try_from(result).or(Err(VaultProgramError::ArithmeticError.into()))
    }

    pub fn get_assets_from_shares(
        &self,
        reserve_balance: u64,
        share_supply: u64,
        share_amount: u64,
        rounding: Rounding,
    ) -> Result<u64> {
        let total_assets = reserve_balance;

        // Bootstrap: no shares exist yet, price is fixed at initial_price.
        if share_supply == 0 {
            let assets = u128::from(share_amount)
                .checked_mul(u128::from(self.initial_price))
                .ok_or(VaultProgramError::ArithmeticError)?;
            return u64::try_from(assets).map_err(|_| VaultProgramError::ArithmeticError.into());
        }
        // Insolvent vault: shares exist but total_assets is zero (losses/rounding/state drift).
        // Return 0 so slippage checks correctly reject redemptions.
        if total_assets == 0 {
            return Ok(0);
        }

        let numerator = u128::from(share_amount)
            .checked_mul(u128::from(total_assets))
            .ok_or(VaultProgramError::ArithmeticError)?;

        let denominator = u128::from(share_supply);

        let result = match rounding {
            Rounding::Up => numerator.div_ceil(denominator),
            Rounding::Down => numerator
                .checked_div(denominator)
                .ok_or(VaultProgramError::ArithmeticError)?,
        };

        u64::try_from(result).map_err(|_| VaultProgramError::ArithmeticError.into())
    }

    pub fn get_deposit_fee_when_minting(&self, assets: u64) -> Result<u64> {
        self.deposit_fee_type()
            .map_or(Ok(0), |(_, fee)| fee.get_deposit_fee_when_minting(assets))
    }

    pub fn get_withdraw_fee_when_redeeming(&self, gross_assets: u64) -> Result<u64> {
        self.withdrawal_fee_type().map_or(Ok(0), |(_, fee)| {
            fee.get_withdraw_fee_when_redeeming(gross_assets)
        })
    }

    pub fn get_deposit_fee(&self, deposit_amount: u64) -> Result<u64> {
        self.deposit_fee_type()
            .map_or(Ok(0), |(_, fee)| fee.get_fee(deposit_amount))
    }

    pub fn get_withdraw_fee(&self, withdraw_amount: u64) -> Result<u64> {
        self.withdrawal_fee_type()
            .map_or(Ok(0), |(_, fee)| fee.get_fee(withdraw_amount))
    }

    pub fn deposit_fee_type(&self) -> Option<(usize, FeeType)> {
        self.extensions
            .iter()
            .enumerate()
            .find_map(|(index, extension)| {
                VaultExtension::as_deposit_fee(extension).map(|fee| (index, fee))
            })
    }

    pub fn withdrawal_fee_type(&self) -> Option<(usize, FeeType)> {
        self.extensions
            .iter()
            .enumerate()
            .find_map(|(index, extension)| {
                VaultExtension::as_withdrawal_fee(extension).map(|fee| (index, fee))
            })
    }

    pub fn deposit_hook_type(&self) -> Option<Pubkey> {
        self.extensions
            .iter()
            .enumerate()
            .find_map(|(_, extension)| {
                VaultExtension::as_deposit_hook(extension).map(|hook_program| hook_program)
            })
    }

    pub fn assert_unpaused_and_initialized(&self) -> Result<()> {
        if !self.initialized {
            return Err(VaultProgramError::UninitializedVault.into());
        }

        if self.paused {
            return Err(VaultProgramError::PausedVault.into());
        }

        Ok(())
    }

    pub fn assert_uninitialized(&self) -> Result<()> {
        if self.initialized {
            return Err(VaultProgramError::VaultAlreadyInitialized.into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn create_vault_config(initial_price: u64) -> VaultConfig {
        VaultConfig {
            asset_mint_address: Pubkey::new_unique(),
            share_mint_address: Pubkey::new_unique(),
            vault_token_account: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            initial_price,
            paused: false,
            initialized: true,
            vault_asset_cap: u64::MAX,
            fee_recipient: Pubkey::new_unique(),
            extensions: vec![],
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
        let vault = create_vault_config(initial_price);
        let shares = vault
            .get_shares_from_assets(total_asset_balance, supply, asset_amount, rounding)
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
        let vault = create_vault_config(initial_price);

        let result =
            vault.get_shares_from_assets(total_asset_balance, supply, asset_amount, rounding);
        assert!(result.is_err());
    }

    #[test]
    fn test_initial_price_variations() {
        let vault_high = create_vault_config(1_000_000_000);

        let shares = vault_high
            .get_shares_from_assets(0, 0, 1, Rounding::Down)
            .unwrap();
        assert_eq!(shares, 1_000_000_000);

        let vault_low = create_vault_config(1);
        let shares_low = vault_low
            .get_shares_from_assets(0, 0, 1_000_000, Rounding::Down)
            .unwrap();
        assert_eq!(shares_low, 1_000_000);
    }

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

    #[test_case(1000,1,500,100,Rounding::Down,200;"Basic calculation rounding down")]
    #[test_case(1000,1,500,100,Rounding::Up,200;"Basic calculation rounding up")]
    #[test_case(1000,1,0,100,Rounding::Down,100;"Zero supply")]
    #[test_case(0,1,500,100,Rounding::Down,0;"Zero assets")]
    #[test_case(0,1,0,0,Rounding::Down,0;"All zeros")]
    #[test_case(1000,1,1000,500,Rounding::Down,500;"Equal supply and total assets")]
    #[test_case(1_000_000_000,1,1_000_000_000,1_000_000,Rounding::Down,1_000_000;"Large values within bounds")]
    #[test_case(3,1,10,1,Rounding::Down,0;"Precision loss rounding down")]
    #[test_case(3,1,10,1,Rounding::Up,1;"Precision loss rounding up")]
    #[test_case(1,1,1,1,Rounding::Down,1;"Single unit")]
    #[test_case(1,1_000_000, 3, 1, Rounding::Down, 0; "ceil differs: 1*1/3 down")]
    #[test_case(1,1_000_000, 3, 1, Rounding::Up, 1; "ceil differs: 1*1/3 up")]
    #[test_case(10_000,1_000_000, 3, 2, Rounding::Down, 6666; "non-clean division down")]
    #[test_case(10_000,1_000_000, 3, 2, Rounding::Up, 6667; "non-clean division up")]
    #[test_case(0,1_000_000, 1_000, 100, Rounding::Down, 0; "Zero assets, positive supply, down")]
    #[test_case(0,1_000_000, 1_000, 100, Rounding::Up, 0; "Zero assets, positive supply, up")]
    #[test_case(10_000,1_000_000, 10_000, 100, Rounding::Down, 100; "1:1 ratio down")]
    #[test_case(10_000,1_000_000, 10_000, 100, Rounding::Up, 100; "1:1 ratio up")]
    #[test_case(2_000,1_000_000, 1_000, 100, Rounding::Down, 200; "2:1 assets:supply down")]
    #[test_case(2_000,1_000_000, 1_000, 100, Rounding::Up, 200; "2:1 assets:supply up")]
    #[test_case(9_999,1_000_000, 10_000, 100, Rounding::Down, 99; "Rounding edge down")]
    #[test_case(9_999,1_000_000, 10_000, 100, Rounding::Up, 100; "Rounding edge up")]
    #[test_case(10_000,1_000_000, 10_000, 0, Rounding::Down, 0; "Zero share_amount returns 0")]
    #[test_case(1_000_000_000,1_000_000, 1_000_000_000, 1_000_000, Rounding::Down, 1_000_000; "Large values 1:1")]
    #[test_case(1_000_000,1_000_000, 1_000_000, 1, Rounding::Down, 1; "Precision small amounts")]
    #[test_case(100,1_000_000, 1_000_000_000, 10, Rounding::Down, 0; "Asymmetric small assets/share down")]
    #[test_case(100,1_000_000, 1_000_000_000, 10, Rounding::Up, 1; "Asymmetric small assets/share up")]
    #[test_case(1_000_000_000,1_000_000, 100, 10, Rounding::Down, 100_000_000; "Asymmetric huge assets/share down")]
    #[test_case(1_000_000_000,1_000_000, 100, 10, Rounding::Up, 100_000_000; "Asymmetric huge assets/share up")]

    fn test_get_assets_from_shares(
        total_asset_amount: u64,
        initial_price: u64,
        supply: u64,
        asset_amount: u64,
        rounding: Rounding,
        expected_amount: u64,
    ) {
        let vault = create_vault_config(initial_price);
        let result =
            vault.get_assets_from_shares(total_asset_amount, supply, asset_amount, rounding);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_amount);
    }

    #[test_case(u64::MAX, 1_000_000, 1, 2, Rounding::Down; "ERROR: result overflows u64")]
    #[test_case(u64::MAX, 1_000_000, 1, u64::MAX, Rounding::Down; "ERROR: result overflows u64 (down)")]
    #[test_case(u64::MAX, 1_000_000, 1, u64::MAX, Rounding::Up; "ERROR: result overflows u64(up)")]
    fn test_get_assets_from_shares_error(
        total_asset_amount: u64,
        initial_price: u64,
        supply: u64,
        asset_amount: u64,
        rounding: Rounding,
    ) {
        let vault = create_vault_config(initial_price);
        let result =
            vault.get_assets_from_shares(total_asset_amount, supply, asset_amount, rounding);

        assert!(result.is_err());
    }

    #[test]
    fn test_rounding_difference() {
        let vault_down = create_vault_config(1);
        let vault_up = create_vault_config(1);

        let result_down = vault_down
            .get_assets_from_shares(1000, 333, 100, Rounding::Down)
            .unwrap();
        let result_up = vault_up
            .get_assets_from_shares(1000, 333, 100, Rounding::Up)
            .unwrap();

        assert!(result_up >= result_down);
    }

    // get_withdraw_fee_when_redeeming: fee = ceil(gross * bps / (MAX_BPS + bps))
    // such that fee/net = bps/MAX_BPS (same rate as withdraw uses on net)
    #[test_case(FeeType::FixedAmount { amount: 50 }, 1_000, 50; "FixedAmount fee")]
    #[test_case(FeeType::Percentage { bps: 0 }, 1_000, 0; "Percentage zero bps")]
    #[test_case(FeeType::Percentage { bps: 100 }, 1_010, 10; "1% on gross=1010 gives fee=10 net=1000")]
    #[test_case(FeeType::Percentage { bps: 500 }, 10_500, 500; "5% on gross=10500 gives fee=500 net=10000")]
    #[test_case(FeeType::Percentage { bps: 10_000 }, 1_000, 500; "100% on gross=1000 gives fee=500 net=500")]
    #[test_case(FeeType::Percentage { bps: 100 }, 1_000, 10; "1% rounds up: ceil(1000*100/10100)=10")]
    fn test_get_withdraw_fee_when_redeeming(fee_type: FeeType, gross: u64, expected_fee: u64) {
        let fee = fee_type.get_withdraw_fee_when_redeeming(gross).unwrap();
        assert_eq!(fee, expected_fee);
        // verify fee rate is on NET (not gross) for Percentage fees
        if let FeeType::Percentage { bps } = fee_type {
            if bps > 0 && fee > 0 {
                let net = gross - fee;
                // fee / net should approximate bps / MAX_BPS (within rounding)
                let fee_rate_times_max_bps = fee * 10_000 / net;
                assert!(fee_rate_times_max_bps <= bps as u64 + 1);
            }
        }
    }

    #[test_case(0,1000,0;"Percentage fee zero bps")]
    #[test_case(100,1000,10;"Percentage fee 100 bps")]
    #[test_case(500,10000,526;"Percentage fee 500 bps")]
    #[test_case(1000,9000,1000;"Percentage fee 1000 bps")]
    #[test_case(9900,100,9900;"Percentage fee 9900 bps")]
    #[test_case(10_000,100,100;"Percentage fee 10_000 bps")]
    fn test_percentage_fee_zero_bps(bps: u16, net_assets: u64, expected_amount: u64) {
        let fee_type = FeeType::Percentage { bps: bps };
        let result = fee_type.get_deposit_fee_when_minting(net_assets).unwrap();
        assert_eq!(result, expected_amount);
    }
}

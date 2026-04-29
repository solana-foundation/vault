use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::{
    self,
    extension::{BaseStateWithExtensions, StateWithExtensions},
};
use vault_common::VaultProgramError;

use crate::error::AsyncVaultError;

/// Validates the extensions on the asset mint to ensure compatibility with the
/// Async Vault program. This is checked during Vault Creation and Deposit/DepositRequest.
/// - TransferFeeConfig can be enabled, but must be 0. Many stablecoins have
/// TransferFeeConfig enabled with a 0 fee, so this is important to support.
pub fn validate_asset_mint_extensions_from_acct_info(mint_acct: &AccountInfo) -> Result<()> {
    if mint_acct.owner != &spl_token_2022::ID {
        return Ok(());
    }
    let mint_data = mint_acct.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    // Validate: Mint has 0 transfer fees
    if let Ok(transfer_fee_config) =
        mint.get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>()
    {
        let clock = Clock::get()?;
        let transfer_fee_bps = u16::from_le_bytes(
            transfer_fee_config
                .get_epoch_fee(clock.epoch)
                .transfer_fee_basis_points
                .0,
        );
        if transfer_fee_bps != 0 {
            return Err(AsyncVaultError::InvalidAssetMintExtensions.into());
        }
    }

    Ok(())
}

/// Converts an asset amount into shares using the supplied NAV.
///
/// `shares = net_amount * 10^decimals / nav`
pub fn calculate_shares(nav: u128, decimals: u8, net_amount: u64) -> Result<u64> {
    let precision = 10u128
        .checked_pow(decimals as u32)
        .ok_or(VaultProgramError::ArithmeticError)?;
    let shares = u128::from(net_amount)
        .checked_mul(precision)
        .ok_or(VaultProgramError::ArithmeticError)?
        .checked_div(nav)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok(u64::try_from(shares).map_err(|_| VaultProgramError::ArithmeticError)?)
}

/// Converts a share amount into assets using the supplied NAV.
///
/// `assets = share_amount * nav / 10^decimals`
pub fn calculate_assets(nav: u128, decimals: u8, share_amount: u64) -> Result<u64> {
    let precision = 10u128
        .checked_pow(decimals as u32)
        .ok_or(VaultProgramError::ArithmeticError)?;
    let assets = u128::from(share_amount)
        .checked_mul(nav)
        .ok_or(VaultProgramError::ArithmeticError)?
        .checked_div(precision)
        .ok_or(VaultProgramError::ArithmeticError)?;
    if assets.eq(&0u128) {
        return Err(VaultProgramError::ArithmeticError.into());
    }
    Ok(u64::try_from(assets).map_err(|_| VaultProgramError::ArithmeticError)?)
}

// TODO consolidate with test_case
// TODO add tests for calculate_assets
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_shares_one_to_one() {
        let shares = calculate_shares(1_000_000, 6, 1_000_000).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn calculate_shares_nav_above_one() {
        let shares = calculate_shares(2_000_000, 6, 2_000_000).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn calculate_shares_fractional_result_truncates() {
        let shares = calculate_shares(3_000_000, 6, 1_000_000).unwrap();
        // 1_000_000 * 1e6 / 3_000_000 = 333_333.333… → truncated to 333_333
        assert_eq!(shares, 333_333);
    }

    #[test]
    fn calculate_shares_zero_amount() {
        let shares = calculate_shares(1_000_000, 6, 0).unwrap();
        assert_eq!(shares, 0);
    }

    #[test]
    fn calculate_shares_different_decimals() {
        // 8 decimals: 1 token = 100_000_000 units
        let shares = calculate_shares(100_000_000, 8, 100_000_000).unwrap();
        assert_eq!(shares, 100_000_000);
    }

    #[test]
    fn calculate_shares_zero_nav_errors() {
        assert!(calculate_shares(0, 6, 1_000_000).is_err());
    }

    #[test]
    fn calculate_shares_large_amount_no_overflow() {
        let shares = calculate_shares(1_000_000, 6, u64::MAX).unwrap();
        assert_eq!(shares, u64::MAX);
    }
}


use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::{
    self,
    extension::{BaseStateWithExtensions, StateWithExtensions},
};

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
      msg!("Validating TransferFeeConfig extension on asset mint {:?}", transfer_fee_config);
        let clock = Clock::get()?;
        let transfer_fee_bps = u16::from_le_bytes(
            transfer_fee_config
                .get_epoch_fee(clock.epoch)
                .transfer_fee_basis_points
                .0,
        );
        msg!("Found transfer fee bps: {}", transfer_fee_bps);
        if transfer_fee_bps != 0 {
            return Err(AsyncVaultError::InvalidAssetMintExtensions.into());
        }
    }

    Ok(())
}

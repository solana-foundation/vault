use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

/// Vault extension: enforces a minimum share amount on redemption requests.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct MinRedemption {
    /// Minimum redemption amount in share token units; requests below this threshold are rejected.
    pub threshold: u64,
}

impl crate::extensions::VaultExtension for MinRedemption {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::MinRedemption;
}

/// Returns `RedemptionAmountBelowMinimum` if the extension is active and `amount < threshold`.
pub fn check_min_redemption_amount(vault_info: &AccountInfo, amount: u64) -> Result<()> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    if let Some(ext) = read_vault_extension::<MinRedemption>(&data)? {
        require!(
            amount >= ext.threshold,
            AsyncVaultError::RedemptionAmountBelowMinimum
        );
    }
    Ok(())
}

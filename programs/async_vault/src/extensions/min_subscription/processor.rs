use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

/// Vault extension: enforces a minimum asset amount on deposit requests.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct MinSubscription {
    /// Minimum deposit amount in asset token units; requests below this threshold are rejected.
    pub threshold: u64,
}

impl crate::extensions::VaultExtension for MinSubscription {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::MinSubscription;
}

/// Returns `SubscriptionAmountBelowMinimum` if the extension is active and `amount < threshold`.
pub fn check_min_subscription_amount(vault_info: &AccountInfo, amount: u64) -> Result<()> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    if let Some(ext) = read_vault_extension::<MinSubscription>(&data)? {
        require!(
            amount >= ext.threshold,
            AsyncVaultError::SubscriptionAmountBelowMinimum
        );
    }
    Ok(())
}

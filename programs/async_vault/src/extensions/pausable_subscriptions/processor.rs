use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

/// Vault extension: pauses or unpauses deposit (subscription) processing.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct PausableSubscription {
    /// 0 = unpaused, 1 = paused.
    pub paused: u8,
}

impl crate::extensions::VaultExtension for PausableSubscription {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::PausableSubscriptions;
}

pub fn check_subscriptions_paused(vault_info: &AccountInfo) -> Result<()> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    if let Some(ext) = read_vault_extension::<PausableSubscription>(&data)? {
        require!(ext.paused == 0, AsyncVaultError::SubscriptionsPaused);
    }
    Ok(())
}

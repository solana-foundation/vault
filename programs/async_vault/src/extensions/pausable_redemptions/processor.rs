use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

/// Vault extension: pauses or unpauses withdrawal (redemption) processing.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct PausableRedemption {
    /// 0 = unpaused, 1 = paused.
    pub paused: u8,
}

impl crate::extensions::VaultExtension for PausableRedemption {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::PausableRedemptions;
}

pub fn check_redemptions_paused(vault_info: &AccountInfo) -> Result<()> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    if let Some(ext) = read_vault_extension::<PausableRedemption>(&data)? {
        require!(ext.paused == 0, AsyncVaultError::RedemptionsPaused);
    }
    Ok(())
}

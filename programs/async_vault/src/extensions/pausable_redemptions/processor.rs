use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PausableRedemption {
    pub paused: bool,
}

impl crate::extensions::VaultExtension for PausableRedemption {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::PausableRedemptions;
}

pub fn check_redemptions_paused(account_data: &[u8]) -> Result<()> {
    if let Some(ext) = read_vault_extension::<PausableRedemption>(account_data)? {
        require!(!ext.paused, AsyncVaultError::RedemptionsPaused);
    }
    Ok(())
}

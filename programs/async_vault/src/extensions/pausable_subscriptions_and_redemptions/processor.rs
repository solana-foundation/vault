use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, TLV_START},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub struct PausableExtension {
    pub paused: bool,
}

pub fn get_pausable_extension(
    account_data: &[u8],
    ext_type: ExtensionType,
) -> Result<Option<PausableExtension>> {
    if account_data.len() <= TLV_START {
        return Ok(None);
    }
    let tlv_data = &account_data[TLV_START..];
    match extensions::get_extension_bytes(tlv_data, ext_type) {
        Some(bytes) => {
            let mut slice = bytes;
            let extension = PausableExtension::deserialize(&mut slice)
                .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;
            Ok(Some(extension))
        }
        None => Ok(None),
    }
}

pub fn get_pausable_subscription(account_data: &[u8]) -> Result<bool> {
    match get_pausable_extension(account_data, ExtensionType::PausableSubcriptionsExtension)? {
        Some(extension) => Ok(extension.paused),
        None => Ok(false),
    }
}

pub fn get_pausable_redemption(account_data: &[u8]) -> Result<bool> {
    match get_pausable_extension(account_data, ExtensionType::PausableRedemptionsExtension)? {
        Some(extension) => Ok(extension.paused),
        None => Ok(false),
    }
}

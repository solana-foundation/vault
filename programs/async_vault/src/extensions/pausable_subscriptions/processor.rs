use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{read_vault_extension, ExtensionType},
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PausableSubscription {
    pub paused: bool,
}

impl crate::extensions::VaultExtension for PausableSubscription {
    const DATA_SIZE: usize = std::mem::size_of::<Self>();
    const EXTENSION_TYPE: ExtensionType = ExtensionType::PausableSubscriptions;
}

pub fn check_subscriptions_paused(account_data: &[u8]) -> Result<()> {
    if let Some(ext) = read_vault_extension::<PausableSubscription>(account_data)? {
        require!(!ext.paused, AsyncVaultError::SubscriptionsPaused);
    }
    Ok(())
}

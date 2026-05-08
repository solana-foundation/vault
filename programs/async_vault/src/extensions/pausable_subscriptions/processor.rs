use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, TLV_START},
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PausableSubscription {
    pub paused: bool,
}

pub fn check_subscriptions_paused(account_data: &[u8]) -> Result<()> {
    if account_data.len() <= TLV_START {
        return Ok(());
    }
    let tlv_data = &account_data[TLV_START..];
    if let Some(bytes) =
        extensions::get_extension_bytes(tlv_data, ExtensionType::PausableSubscriptions)
    {
        if !bytes.is_empty() && bytes[0] == 1 {
            return Err(AsyncVaultError::SubscriptionsPaused.into());
        }
    }
    Ok(())
}

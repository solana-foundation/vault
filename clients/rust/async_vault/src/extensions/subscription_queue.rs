use borsh::BorshDeserialize;

use super::{get_extension_bytes, ExtensionType, VAULT_TLV_START};

/// State of the SubscriptionQueue vault extension.
#[derive(BorshDeserialize)]
pub struct SubscriptionQueue {
    pub all_time_total_subscription_requests: u64,
    pub last_processed_subscription_request_index: u64,
}

/// Returns the [`SubscriptionQueue`] extension state from raw vault account data,
/// or `None` if the extension is not present.
pub fn get_state(vault_data: &[u8]) -> Option<SubscriptionQueue> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(
        &vault_data[VAULT_TLV_START..],
        ExtensionType::SubscriptionQueue,
    )?;
    SubscriptionQueue::try_from_slice(bytes).ok()
}

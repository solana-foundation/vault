use borsh::BorshDeserialize;

use super::{
    get_extension_bytes, get_request_extension_bytes, ExtensionType, RequestExtensionType,
    VAULT_TLV_START,
};

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

/// State of the SubscriptionQueueRequest extension on a deposit request account.
#[derive(BorshDeserialize)]
pub struct SubscriptionQueueRequest {
    /// Monotonically increasing ID matching `all_time_total_subscription_requests`
    /// at the time this request was created.
    pub id: u64,
}

/// Returns the [`SubscriptionQueueRequest`] extension from raw request account data,
/// or `None` if the extension is not present.
pub fn get_request_state(request_data: &[u8]) -> Option<SubscriptionQueueRequest> {
    let bytes =
        get_request_extension_bytes(request_data, RequestExtensionType::SubscriptionQueueRequest)?;
    SubscriptionQueueRequest::try_from_slice(bytes).ok()
}

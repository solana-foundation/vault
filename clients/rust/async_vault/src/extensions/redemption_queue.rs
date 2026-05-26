use borsh::BorshDeserialize;

use super::{
    get_extension_bytes, get_request_extension_bytes, ExtensionType, RequestExtensionType,
    VAULT_TLV_START,
};

/// State of the RedemptionQueue vault extension.
#[derive(BorshDeserialize)]
pub struct RedemptionQueue {
    pub all_time_total_redemption_requests: u64,
    pub last_processed_redemption_request_index: u64,
}

/// Returns the [`RedemptionQueue`] extension state from raw vault account data,
/// or `None` if the extension is not present.
pub fn get_state(vault_data: &[u8]) -> Option<RedemptionQueue> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(
        &vault_data[VAULT_TLV_START..],
        ExtensionType::RedemptionQueue,
    )?;
    RedemptionQueue::try_from_slice(bytes).ok()
}

/// State of the RedemptionQueueRequest extension on a redeem request account.
#[derive(BorshDeserialize)]
pub struct RedemptionQueueRequest {
    /// Monotonically increasing ID matching `all_time_total_redemption_requests`
    /// at the time this request was created.
    pub id: u64,
}

/// Returns the [`RedemptionQueueRequest`] extension from raw request account data,
/// or `None` if the extension is not present.
pub fn get_request_state(request_data: &[u8]) -> Option<RedemptionQueueRequest> {
    let bytes =
        get_request_extension_bytes(request_data, RequestExtensionType::RedemptionQueueRequest)?;
    RedemptionQueueRequest::try_from_slice(bytes).ok()
}

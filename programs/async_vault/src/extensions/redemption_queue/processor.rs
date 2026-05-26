use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{
        fifo_queue::{FifoQueue, QueueRequest},
        request_extensions::{RequestExtension, RequestExtensionType},
        ExtensionType, VaultExtension,
    },
};

/// Vault extension: tracks FIFO ordering counters for redeem requests.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct RedemptionQueue {
    /// Total number of redeem requests ever created on this vault.
    pub all_time_total_redemption_requests: u64,
    /// Index of the last redeem request that was approved or rejected.
    pub last_processed_redemption_request_index: u64,
}

impl VaultExtension for RedemptionQueue {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::RedemptionQueue;
}

impl FifoQueue for RedemptionQueue {
    const OUT_OF_ORDER_ERROR: AsyncVaultError = AsyncVaultError::RedemptionQueueOutOfOrder;

    fn total(&self) -> u64 {
        self.all_time_total_redemption_requests
    }

    fn last_processed(&self) -> u64 {
        self.last_processed_redemption_request_index
    }

    fn set_last_processed(&mut self, id: u64) {
        self.last_processed_redemption_request_index = id;
    }

    fn increment_total(&mut self) -> Result<u64> {
        self.all_time_total_redemption_requests =
            self.all_time_total_redemption_requests.wrapping_add(1);
        Ok(self.all_time_total_redemption_requests)
    }
}

/// Request extension: the sequential redeem request ID assigned at creation.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct RedemptionQueueRequest {
    /// Sequential ID assigned at creation time. Wraps to 0 after `u64::MAX`; uniqueness
    /// is not guaranteed, but FIFO ordering is preserved within any active request window.
    pub id: u64,
}

impl RequestExtension for RedemptionQueueRequest {
    const EXTENSION_TYPE: RequestExtensionType = RequestExtensionType::RedemptionQueueRequest;
}

impl QueueRequest for RedemptionQueueRequest {
    fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_total_normal() {
        let mut queue = RedemptionQueue {
            all_time_total_redemption_requests: 0,
            last_processed_redemption_request_index: 0,
        };
        assert_eq!(queue.increment_total().unwrap(), 1);
        assert_eq!(queue.all_time_total_redemption_requests, 1);
    }

    #[test]
    fn test_increment_total_wraps_at_max() {
        let mut queue = RedemptionQueue {
            all_time_total_redemption_requests: u64::MAX,
            last_processed_redemption_request_index: 0,
        };
        assert_eq!(queue.increment_total().unwrap(), 0);
        assert_eq!(queue.all_time_total_redemption_requests, 0);
    }
}

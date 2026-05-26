use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

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
        self.all_time_total_redemption_requests = self
            .all_time_total_redemption_requests
            .checked_add(1)
            .ok_or(VaultProgramError::ArithmeticError)?;
        Ok(self.all_time_total_redemption_requests)
    }
}

/// Request extension: the sequential redeem request ID assigned at creation.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct RedemptionQueueRequest {
    /// Monotonically increasing ID matching `all_time_total_redemption_requests`
    /// at the time this request was created.
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

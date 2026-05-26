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

/// Vault extension: tracks FIFO ordering counters for deposit requests.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct SubscriptionQueue {
    /// Total number of deposit requests ever created on this vault.
    pub all_time_total_subscription_requests: u64,
    /// Index of the last deposit request that was approved or rejected.
    pub last_processed_subscription_request_index: u64,
}

impl VaultExtension for SubscriptionQueue {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::SubscriptionQueue;
}

impl FifoQueue for SubscriptionQueue {
    const OUT_OF_ORDER_ERROR: AsyncVaultError = AsyncVaultError::SubscriptionQueueOutOfOrder;

    fn total(&self) -> u64 {
        self.all_time_total_subscription_requests
    }

    fn last_processed(&self) -> u64 {
        self.last_processed_subscription_request_index
    }

    fn set_last_processed(&mut self, id: u64) {
        self.last_processed_subscription_request_index = id;
    }

    fn increment_total(&mut self) -> Result<u64> {
        self.all_time_total_subscription_requests = self
            .all_time_total_subscription_requests
            .checked_add(1)
            .ok_or(VaultProgramError::ArithmeticError)?;
        Ok(self.all_time_total_subscription_requests)
    }
}

/// Request extension: the sequential deposit request ID assigned at creation.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct SubscriptionQueueRequest {
    /// Monotonically increasing ID matching `all_time_total_subscription_requests`
    /// at the time this request was created.
    pub id: u64,
}

impl RequestExtension for SubscriptionQueueRequest {
    const EXTENSION_TYPE: RequestExtensionType = RequestExtensionType::SubscriptionQueueRequest;
}

impl QueueRequest for SubscriptionQueueRequest {
    fn id(&self) -> u64 {
        self.id
    }
}

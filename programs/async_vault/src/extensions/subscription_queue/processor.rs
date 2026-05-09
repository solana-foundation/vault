use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    extensions::{
        read_vault_extension,
        request_extensions::{read_request_extension, RequestExtension, RequestExtensionType},
        update_vault_extension, ExtensionType, VaultExtension,
    },
};

/// Vault extension: tracks FIFO ordering counters for deposit requests.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SubscriptionQueue {
    /// Total number of deposit requests ever created on this vault.
    pub all_time_total_subscription_requests: u64,
    /// Index of the last deposit request that was approved or rejected.
    pub last_processed_subscription_request_index: u64,
}

impl VaultExtension for SubscriptionQueue {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::SubscriptionQueue;
}

/// Request extension: the sequential deposit request ID assigned at creation.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SubscriptionQueueRequest {
    /// Monotonically increasing ID matching `all_time_total_subscription_requests`
    /// at the time this request was created.
    pub id: u64,
}

impl RequestExtension for SubscriptionQueueRequest {
    const EXTENSION_TYPE: RequestExtensionType = RequestExtensionType::SubscriptionQueueRequest;
}

/// Validates FIFO ordering for a deposit request and returns the updated vault extension.
///
/// Returns `Ok(None)` if the vault does not have the SubscriptionQueue extension, meaning
/// no ordering constraint applies. Returns `Ok(Some(updated_queue))` when the check passes
/// and the caller must write the updated extension back to the vault via
/// [`update_vault_extension`].
///
/// # Errors
/// - [`AsyncVaultError::UninitializedExtension`] if the vault has SubscriptionQueue but the request
///   account lacks the corresponding [`SubscriptionQueueRequest`] extension.
/// - [`AsyncVaultError::SubscriptionQueueOutOfOrder`] if the request's ID does not equal
///   `last_processed_subscription_request_index + 1`.
pub fn check_and_advance_subscription_queue(
    vault_data: &[u8],
    request_data: &[u8],
) -> Result<Option<SubscriptionQueue>> {
    let Some(mut queue) = read_vault_extension::<SubscriptionQueue>(vault_data)? else {
        return Ok(None);
    };

    let req_ext = read_request_extension::<SubscriptionQueueRequest>(request_data)?
        .ok_or(AsyncVaultError::UninitializedExtension)?;

    let expected = queue
        .last_processed_subscription_request_index
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        req_ext.id == expected,
        AsyncVaultError::SubscriptionQueueOutOfOrder
    );

    queue.last_processed_subscription_request_index = req_ext.id;
    Ok(Some(queue))
}

/// Increments the vault's `all_time_total_subscription_requests` counter and returns
/// the new request ID to be stored on the request account extension.
pub fn next_subscription_request_id(
    vault_info: &AccountInfo,
    vault_data: &[u8],
) -> Result<Option<u64>> {
    let Some(mut queue) = read_vault_extension::<SubscriptionQueue>(vault_data)? else {
        return Ok(None);
    };

    queue.all_time_total_subscription_requests = queue
        .all_time_total_subscription_requests
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    let id = queue.all_time_total_subscription_requests;
    update_vault_extension(vault_info, &queue)?;
    Ok(Some(id))
}

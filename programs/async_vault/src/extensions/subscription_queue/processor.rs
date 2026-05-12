use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    extensions::{
        read_vault_extension,
        request_extensions::{read_request_extension, RequestExtension, RequestExtensionType},
        tlv::{get_extension_bytes_raw_mut, TLV_START},
        update_vault_extension, ExtensionType, VaultExtension,
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

/// Validates FIFO ordering for a deposit request and, if the SubscriptionQueue extension is
/// present, advances `last_processed_subscription_request_index` and writes the updated
/// extension back to the vault account.
///
/// Returns `Ok(())` whether or not the extension is present; no ordering constraint applies
/// when the extension is absent.
///
/// # Errors
/// - [`AsyncVaultError::UninitializedExtension`] if the vault has SubscriptionQueue but the request
///   account lacks the corresponding [`SubscriptionQueueRequest`] extension.
/// - [`AsyncVaultError::SubscriptionQueueOutOfOrder`] if the request's ID does not equal
///   `last_processed_subscription_request_index + 1`.
pub fn check_and_advance_subscription_queue(
    vault_info: &AccountInfo,
    request_info: &AccountInfo,
) -> Result<()> {
    let vault_data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let Some(mut queue) = read_vault_extension::<SubscriptionQueue>(&vault_data)? else {
        return Ok(());
    };
    drop(vault_data);

    let request_data = request_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let req_ext = read_request_extension::<SubscriptionQueueRequest>(&request_data)?
        .ok_or(AsyncVaultError::UninitializedExtension)?;
    drop(request_data);

    let expected = queue
        .last_processed_subscription_request_index
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(
        req_ext.id == expected,
        AsyncVaultError::SubscriptionQueueOutOfOrder
    );

    queue.last_processed_subscription_request_index = req_ext.id;
    update_vault_extension(vault_info, &queue)
}

/// Increments the vault's `all_time_total_subscription_requests` counter in-place and
/// returns the new request ID to be stored on the request account extension.
pub fn next_subscription_request_id(vault_info: &AccountInfo) -> Result<Option<u64>> {
    let mut data = vault_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    if data.len() <= TLV_START {
        return Ok(None);
    }

    let tlv_data = &mut data[TLV_START..];
    let Some(value_bytes) =
        get_extension_bytes_raw_mut(tlv_data, ExtensionType::SubscriptionQueue as u16)
    else {
        return Ok(None);
    };

    // try_from_bytes_mut would avoid the copy but requires 8-byte alignment, which TLV
    // value offsets don't guarantee. `try_pod_read_unaligned` + `copy_from_slice` handles any
    // alignment safely.
    let mut queue: SubscriptionQueue = bytemuck::try_pod_read_unaligned(value_bytes)
        .map_err(|_| AsyncVaultError::InvalidExtensionData)?;
    queue.all_time_total_subscription_requests = queue
        .all_time_total_subscription_requests
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;
    let id = queue.all_time_total_subscription_requests;
    value_bytes.copy_from_slice(bytemuck::bytes_of(&queue));

    Ok(Some(id))
}

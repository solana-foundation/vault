use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

use crate::{
    error::AsyncVaultError,
    extensions::{
        read_vault_extension,
        request_extensions::{read_request_extension, RequestExtension},
        tlv::{get_extension_bytes_raw_mut, TLV_START},
        update_vault_extension, VaultExtension,
    },
};

/// Shared behavior for FIFO queue vault extensions. Implemented by both
/// [`SubscriptionQueue`] and [`RedemptionQueue`]
pub trait FifoQueue: VaultExtension {
    /// Error returned when a request is processed out of FIFO order.
    const OUT_OF_ORDER_ERROR: AsyncVaultError;

    /// All-time total requests ever created on this vault.
    fn total(&self) -> u64;
    /// Index of the last request that was approved or rejected.
    fn last_processed(&self) -> u64;
    /// Advance the last-processed pointer to `id`.
    fn set_last_processed(&mut self, id: u64);
    /// Increment the all-time total and return the new ID.
    fn increment_total(&mut self) -> Result<u64>;
}

/// Adds `id()` access to FIFO queue request extensions.
pub trait QueueRequest: RequestExtension {
    fn id(&self) -> u64;
}

/// Increments the vault queue's all-time total counter in-place and returns the new ID.
///
/// Uses an unaligned read+copy pattern because TLV value offsets don't guarantee 8-byte
/// alignment, so `try_from_bytes_mut` cannot be used safely here.
///
/// Returns `Ok(None)` if the queue extension is not present on the vault.
pub fn next_queue_request_id<Q: FifoQueue>(vault_info: &AccountInfo) -> Result<Option<u64>> {
    let mut data = vault_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    if data.len() <= TLV_START {
        return Ok(None);
    }
    let tlv_data = &mut data[TLV_START..];
    let Some(value_bytes) = get_extension_bytes_raw_mut(tlv_data, Q::EXTENSION_TYPE as u16) else {
        return Ok(None);
    };
    let mut queue: Q = bytemuck::try_pod_read_unaligned(value_bytes)
        .map_err(|_| AsyncVaultError::InvalidExtensionData)?;
    let id = queue.increment_total()?;
    value_bytes.copy_from_slice(bytemuck::bytes_of(&queue));
    Ok(Some(id))
}

/// Validates FIFO ordering for a queued request and, if the extension is present, advances
/// `last_processed` on the vault and writes the updated extension back.
///
/// Returns `Ok(())` whether or not the extension is present; no ordering constraint applies
/// when the extension is absent.
///
/// # Errors
/// - [`AsyncVaultError::UninitializedExtension`] if the vault has the queue but the request account
///   lacks the corresponding request extension.
/// - `Q::OUT_OF_ORDER_ERROR` if the request's ID does not equal `last_processed + 1`.
pub fn check_and_advance_queue<Q: FifoQueue, R: QueueRequest>(
    vault_info: &AccountInfo,
    request_info: &AccountInfo,
) -> Result<()> {
    let vault_data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let Some(mut queue) = read_vault_extension::<Q>(&vault_data)? else {
        return Ok(());
    };
    drop(vault_data);

    let request_data = request_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let req_ext = read_request_extension::<R>(&request_data)?
        .ok_or(AsyncVaultError::UninitializedExtension)?;
    drop(request_data);

    let expected = queue
        .last_processed()
        .checked_add(1)
        .ok_or(VaultProgramError::ArithmeticError)?;

    require!(req_ext.id() == expected, Q::OUT_OF_ORDER_ERROR);

    queue.set_last_processed(req_ext.id());
    update_vault_extension(vault_info, &queue)
}

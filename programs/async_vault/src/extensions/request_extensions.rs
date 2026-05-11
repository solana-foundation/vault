use anchor_lang::prelude::*;

use crate::state::Request;

use super::{
    tlv::{get_extension_bytes_raw, has_extension_raw, tlv_used_len, write_extension_raw},
    ExtensionType, TLV_HEADER_SIZE, TLV_START,
};

/// Offset at which TLV extension data begins in a Request account.
pub const REQUEST_TLV_START: usize = 8 + Request::INIT_SPACE;

/// Discriminants for extensions stored in Request account TLV data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum RequestExtensionType {
    SubscriptionQueueRequest = 1,
}

impl RequestExtensionType {
    pub fn try_from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::SubscriptionQueueRequest),
            _ => None,
        }
    }

    pub const fn data_len(&self) -> usize {
        match self {
            Self::SubscriptionQueueRequest => 8,
        }
    }
}

/// Implemented by every extension data struct targeting a Request account.
///
/// Mirrors [`VaultExtension`](super::VaultExtension) but uses [`RequestExtensionType`]
/// discriminants and writes into the Request account's TLV region.
pub trait RequestExtension: bytemuck::Pod + bytemuck::Zeroable {
    /// The TLV discriminant that identifies this extension in request account data.
    const EXTENSION_TYPE: RequestExtensionType;
    /// Byte count of this extension's data payload.
    const DATA_SIZE: usize = Self::EXTENSION_TYPE.data_len();
    /// Total bytes consumed in the TLV region: `TLV_HEADER_SIZE + DATA_SIZE`.
    const TLV_SIZE: usize = TLV_HEADER_SIZE + Self::DATA_SIZE;
}

/// Writes a new TLV extension entry to a request account's extension region.
pub fn init_request_extension<E: RequestExtension>(
    request_info: &AccountInfo,
    value: &E,
) -> Result<()> {
    let mut data = request_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let tlv_data = &mut data[REQUEST_TLV_START..];
    let write_offset = tlv_used_len(tlv_data);
    write_extension_raw(
        tlv_data,
        write_offset,
        E::EXTENSION_TYPE as u16,
        E::DATA_SIZE,
        bytemuck::bytes_of(value),
    )
}

/// Returns `true` if the request account data contains the given extension type.
pub fn has_request_extension<E: RequestExtension>(account_data: &[u8]) -> bool {
    if account_data.len() <= REQUEST_TLV_START {
        return false;
    }
    let tlv_data = &account_data[REQUEST_TLV_START..];
    has_extension_raw(tlv_data, E::EXTENSION_TYPE as u16)
}

/// Reads a request extension from raw account data.
///
/// Returns `Ok(None)` if the account has no TLV region or the extension type is absent.
pub fn read_request_extension<E: RequestExtension>(account_data: &[u8]) -> Result<Option<E>> {
    if account_data.len() <= REQUEST_TLV_START {
        return Ok(None);
    }
    let tlv_data = &account_data[REQUEST_TLV_START..];
    Ok(get_extension_bytes_raw(tlv_data, E::EXTENSION_TYPE as u16)
        .map(|bytes| bytemuck::pod_read_unaligned::<E>(bytes)))
}

/// Computes the total TLV space required for request extensions based on the vault's
/// active extensions. Iterates all vault extensions that have a corresponding request
/// extension and sums their sizes.
///
/// Returns 0 if the vault has no extensions that require a request-side counterpart.
pub fn compute_request_extension_space(vault_info: &AccountInfo) -> usize {
    let Ok(data) = vault_info.data.try_borrow() else {
        return 0;
    };
    if data.len() <= TLV_START {
        return 0;
    }
    let tlv = &data[TLV_START..];
    let mut space = 0;
    if has_extension_raw(tlv, ExtensionType::SubscriptionQueue as u16) {
        space += TLV_HEADER_SIZE + RequestExtensionType::SubscriptionQueueRequest.data_len();
    }
    space
}

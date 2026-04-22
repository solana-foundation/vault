use anchor_lang::prelude::*;

use crate::error::AsyncVaultError;

use super::{ExtensionType, TLV_HEADER_SIZE};

pub fn get_extension_bytes(tlv_data: &[u8], ext_type: ExtensionType) -> Option<&[u8]> {
    let mut offset = 0;

    while offset + TLV_HEADER_SIZE <= tlv_data.len() {
        let entry_type = u16::from_le_bytes([tlv_data[offset], tlv_data[offset + 1]]);
        let entry_len = u16::from_le_bytes([tlv_data[offset + 2], tlv_data[offset + 3]]) as usize;

        let value_end = offset + TLV_HEADER_SIZE + entry_len;
        if value_end > tlv_data.len() {
            return None;
        }

        if entry_type == ext_type as u16 {
            return Some(&tlv_data[offset + TLV_HEADER_SIZE..value_end]);
        }

        offset = value_end;
    }

    None
}

pub fn has_extension(tlv_data: &[u8], ext_type: ExtensionType) -> bool {
    get_extension_bytes(tlv_data, ext_type).is_some()
}

pub fn write_extension(
    tlv_data: &mut [u8],
    write_offset: usize,
    ext_type: ExtensionType,
    value: &[u8],
) -> Result<()> {
    let data_len = ext_type.data_len();
    require!(
        value.len() <= data_len,
        AsyncVaultError::InvalidExtensionData
    );
    require!(
        write_offset + TLV_HEADER_SIZE + data_len <= tlv_data.len(),
        AsyncVaultError::InvalidExtensionData
    );

    tlv_data[write_offset..write_offset + 2].copy_from_slice(&(ext_type as u16).to_le_bytes());
    tlv_data[write_offset + 2..write_offset + 4].copy_from_slice(&(data_len as u16).to_le_bytes());

    let value_start = write_offset + TLV_HEADER_SIZE;
    tlv_data[value_start..value_start + value.len()].copy_from_slice(value);
    for byte in &mut tlv_data[value_start + value.len()..value_start + data_len] {
        *byte = 0;
    }

    Ok(())
}

pub fn update_extension(
    tlv_data: &mut [u8],
    ext_type: ExtensionType,
    new_value: &[u8],
) -> Result<()> {
    let data_len = ext_type.data_len();
    require!(
        new_value.len() <= data_len,
        AsyncVaultError::InvalidExtensionData
    );

    let mut offset = 0;
    while offset + TLV_HEADER_SIZE <= tlv_data.len() {
        let entry_type = u16::from_le_bytes([tlv_data[offset], tlv_data[offset + 1]]);
        let entry_len = u16::from_le_bytes([tlv_data[offset + 2], tlv_data[offset + 3]]) as usize;

        let value_end = offset + TLV_HEADER_SIZE + entry_len;
        if value_end > tlv_data.len() {
            return Err(AsyncVaultError::UninitializedExtension.into());
        }

        if entry_type == ext_type as u16 {
            let value_start = offset + TLV_HEADER_SIZE;
            tlv_data[value_start..value_start + new_value.len()].copy_from_slice(new_value);
            for byte in &mut tlv_data[value_start + new_value.len()..value_end] {
                *byte = 0;
            }
            return Ok(());
        }

        offset = value_end;
    }

    Err(AsyncVaultError::UninitializedExtension.into())
}

pub fn tlv_used_len(tlv_data: &[u8]) -> usize {
    let mut offset = 0;

    while offset + TLV_HEADER_SIZE <= tlv_data.len() {
        let entry_len = match tlv_data[offset + 2..offset + 4].try_into() {
            Ok(bytes) => u16::from_le_bytes(bytes) as usize,
            Err(_) => break,
        };

        let next = offset + TLV_HEADER_SIZE + entry_len;
        if next > tlv_data.len() {
            break;
        }

        offset = next;
    }

    offset
}

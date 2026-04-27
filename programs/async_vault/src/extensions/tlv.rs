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
        let entry_type = u16::from_le_bytes([tlv_data[offset], tlv_data[offset + 1]]);
        if entry_type == 0 {
            break;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tlv_entry(ext_type: ExtensionType, value: &[u8]) -> Vec<u8> {
        let data_len = ext_type.data_len();
        let mut buf = Vec::with_capacity(TLV_HEADER_SIZE + data_len);
        buf.extend_from_slice(&(ext_type as u16).to_le_bytes());
        buf.extend_from_slice(&(data_len as u16).to_le_bytes());
        buf.extend_from_slice(value);
        buf.resize(TLV_HEADER_SIZE + data_len, 0);
        buf
    }

    // ---- get_extension_bytes ----

    #[test]
    fn get_extension_bytes_empty_buffer() {
        assert_eq!(get_extension_bytes(&[], ExtensionType::DepositFee), None);
    }

    #[test]
    fn get_extension_bytes_not_found() {
        let buf = make_tlv_entry(ExtensionType::DepositFee, &[1, 2, 3]);
        assert_eq!(
            get_extension_bytes(&buf, ExtensionType::WithdrawalFee),
            None
        );
    }

    #[test]
    fn get_extension_bytes_found() {
        let value = &[10, 20, 30, 40, 50, 60, 70, 80, 90];
        let buf = make_tlv_entry(ExtensionType::DepositFee, value);
        let result = get_extension_bytes(&buf, ExtensionType::DepositFee).unwrap();
        assert_eq!(result, value);
    }

    #[test]
    fn get_extension_bytes_multiple_entries() {
        let dep_value = [1; 9];
        let wd_value = [2; 9];
        let mut buf = make_tlv_entry(ExtensionType::DepositFee, &dep_value);
        buf.extend_from_slice(&make_tlv_entry(ExtensionType::WithdrawalFee, &wd_value));

        assert_eq!(
            get_extension_bytes(&buf, ExtensionType::DepositFee).unwrap(),
            &dep_value
        );
        assert_eq!(
            get_extension_bytes(&buf, ExtensionType::WithdrawalFee).unwrap(),
            &wd_value
        );
    }

    #[test]
    fn get_extension_bytes_header_too_short() {
        assert_eq!(
            get_extension_bytes(&[0, 1, 2], ExtensionType::DepositFee),
            None
        );
    }

    // ---- has_extension ----

    #[test]
    fn has_extension_true() {
        let buf = make_tlv_entry(ExtensionType::WithdrawalFee, &[0; 9]);
        assert!(has_extension(&buf, ExtensionType::WithdrawalFee));
    }

    #[test]
    fn has_extension_false() {
        let buf = make_tlv_entry(ExtensionType::WithdrawalFee, &[0; 9]);
        assert!(!has_extension(&buf, ExtensionType::DepositFee));
    }

    #[test]
    fn has_extension_empty() {
        assert!(!has_extension(&[], ExtensionType::DepositFee));
    }

    // ---- write_extension ----

    #[test]
    fn write_extension_success() {
        let ext = ExtensionType::DepositFee;
        let value = &[5, 6, 7];
        let mut buf = vec![0u8; TLV_HEADER_SIZE + ext.data_len()];

        write_extension(&mut buf, 0, ext, value).unwrap();

        assert_eq!(u16::from_le_bytes([buf[0], buf[1]]), ext as u16);
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), ext.data_len() as u16);
        assert_eq!(&buf[TLV_HEADER_SIZE..TLV_HEADER_SIZE + 3], value);
        assert!(buf[TLV_HEADER_SIZE + 3..].iter().all(|&b| b == 0));
    }

    #[test]
    fn write_extension_at_offset() {
        let ext = ExtensionType::WithdrawalFee;
        let entry_size = TLV_HEADER_SIZE + ext.data_len();
        let mut buf = vec![0xFFu8; entry_size * 2];

        write_extension(&mut buf, entry_size, ext, &[42]).unwrap();

        assert_eq!(buf[entry_size], ext as u16 as u8);
        assert_eq!(buf[entry_size + TLV_HEADER_SIZE], 42);
    }

    #[test]
    fn write_extension_value_too_long() {
        let ext = ExtensionType::DepositFee;
        let oversized = vec![0u8; ext.data_len() + 1];
        let mut buf = vec![0u8; TLV_HEADER_SIZE + ext.data_len() + 1];

        assert!(write_extension(&mut buf, 0, ext, &oversized).is_err());
    }

    #[test]
    fn write_extension_buffer_too_small() {
        let ext = ExtensionType::DepositFee;
        let mut buf = vec![0u8; TLV_HEADER_SIZE + ext.data_len() - 1];

        assert!(write_extension(&mut buf, 0, ext, &[1]).is_err());
    }

    #[test]
    fn write_extension_zero_pads() {
        let ext = ExtensionType::DepositFee;
        let mut buf = vec![0xFFu8; TLV_HEADER_SIZE + ext.data_len()];

        write_extension(&mut buf, 0, ext, &[1]).unwrap();

        let padding = &buf[TLV_HEADER_SIZE + 1..TLV_HEADER_SIZE + ext.data_len()];
        assert!(padding.iter().all(|&b| b == 0));
    }

    // ---- update_extension ----

    #[test]
    fn update_extension_success() {
        let ext = ExtensionType::DepositFee;
        let mut buf = make_tlv_entry(ext, &[1; 9]);

        let new_value = &[9, 8, 7, 6, 5, 4, 3, 2, 1];
        update_extension(&mut buf, ext, new_value).unwrap();

        assert_eq!(&buf[TLV_HEADER_SIZE..TLV_HEADER_SIZE + 9], new_value);
    }

    #[test]
    fn update_extension_shorter_value_zero_pads() {
        let ext = ExtensionType::DepositFee;
        let mut buf = make_tlv_entry(ext, &[0xFF; 9]);

        update_extension(&mut buf, ext, &[42]).unwrap();

        assert_eq!(buf[TLV_HEADER_SIZE], 42);
        assert!(buf[TLV_HEADER_SIZE + 1..TLV_HEADER_SIZE + 9]
            .iter()
            .all(|&b| b == 0));
    }

    #[test]
    fn update_extension_not_found() {
        let mut buf = make_tlv_entry(ExtensionType::DepositFee, &[1; 9]);
        assert!(update_extension(&mut buf, ExtensionType::WithdrawalFee, &[0]).is_err());
    }

    #[test]
    fn update_extension_value_too_long() {
        let ext = ExtensionType::DepositFee;
        let mut buf = make_tlv_entry(ext, &[1; 9]);
        let oversized = vec![0u8; ext.data_len() + 1];

        assert!(update_extension(&mut buf, ext, &oversized).is_err());
    }

    #[test]
    fn update_extension_truncated_buffer() {
        let ext = ExtensionType::DepositFee;
        let mut buf = make_tlv_entry(ext, &[1; 9]);
        buf[2..4].copy_from_slice(&100u16.to_le_bytes());

        assert!(update_extension(&mut buf, ext, &[0]).is_err());
    }

    #[test]
    fn update_extension_second_entry() {
        let ext_a = ExtensionType::DepositFee;
        let ext_b = ExtensionType::WithdrawalFee;
        let mut buf = make_tlv_entry(ext_a, &[1; 9]);
        buf.extend_from_slice(&make_tlv_entry(ext_b, &[2; 9]));

        update_extension(&mut buf, ext_b, &[99]).unwrap();

        assert_eq!(get_extension_bytes(&buf, ext_a).unwrap(), &[1; 9]);
        let b_data = get_extension_bytes(&buf, ext_b).unwrap();
        assert_eq!(b_data[0], 99);
        assert!(b_data[1..].iter().all(|&b| b == 0));
    }

    // ---- tlv_used_len ----

    #[test]
    fn tlv_used_len_empty() {
        assert_eq!(tlv_used_len(&[]), 0);
    }

    #[test]
    fn tlv_used_len_single_entry() {
        let buf = make_tlv_entry(ExtensionType::DepositFee, &[1; 9]);
        assert_eq!(tlv_used_len(&buf), TLV_HEADER_SIZE + 9);
    }

    #[test]
    fn tlv_used_len_two_entries() {
        let mut buf = make_tlv_entry(ExtensionType::DepositFee, &[1; 9]);
        buf.extend_from_slice(&make_tlv_entry(ExtensionType::WithdrawalFee, &[2; 9]));
        assert_eq!(tlv_used_len(&buf), 2 * (TLV_HEADER_SIZE + 9));
    }

    #[test]
    fn tlv_used_len_stops_at_zero_type() {
        let mut buf = make_tlv_entry(ExtensionType::DepositFee, &[1; 9]);
        buf.extend_from_slice(&[0; TLV_HEADER_SIZE + 9]);
        assert_eq!(tlv_used_len(&buf), TLV_HEADER_SIZE + 9);
    }

    #[test]
    fn tlv_used_len_truncated_entry() {
        let mut buf = make_tlv_entry(ExtensionType::DepositFee, &[1; 9]);
        buf.extend_from_slice(&(ExtensionType::WithdrawalFee as u16).to_le_bytes());
        buf.extend_from_slice(&100u16.to_le_bytes());
        assert_eq!(tlv_used_len(&buf), TLV_HEADER_SIZE + 9);
    }

    #[test]
    fn tlv_used_len_header_too_short() {
        assert_eq!(tlv_used_len(&[1, 0, 5]), 0);
    }
}

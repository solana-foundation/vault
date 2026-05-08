pub mod fee;
pub mod pausable_subscriptions;

pub(super) const VAULT_TLV_START: usize = 272;

const TLV_HEADER_SIZE: usize = 4;

#[derive(Clone, Copy)]
#[repr(u16)]
pub(super) enum ExtensionType {
    DepositFee = 1,
    WithdrawalFee = 2,
    PausableSubscriptions = 3,
}

pub(super) fn get_extension_bytes(tlv_data: &[u8], ext_type: ExtensionType) -> Option<&[u8]> {
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

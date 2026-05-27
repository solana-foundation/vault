use super::{get_extension_bytes, ExtensionType, VAULT_TLV_START};

pub struct MinRedemption {
    pub threshold: u64,
}

pub fn get_state(vault_data: &[u8]) -> Option<MinRedemption> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(&vault_data[VAULT_TLV_START..], ExtensionType::MinRedemption)?;
    if bytes.len() < 8 {
        return None;
    }
    Some(MinRedemption {
        threshold: u64::from_le_bytes(bytes[..8].try_into().ok()?),
    })
}

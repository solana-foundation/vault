use super::{get_extension_bytes, ExtensionType, VAULT_TLV_START};

pub struct PausableRedemptions {
    pub paused: bool,
}

pub fn get_state(vault_data: &[u8]) -> Option<PausableRedemptions> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(
        &vault_data[VAULT_TLV_START..],
        ExtensionType::PausableRedemptions,
    )?;
    if bytes.is_empty() {
        return None;
    }
    Some(PausableRedemptions {
        paused: bytes[0] != 0,
    })
}

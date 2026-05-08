use super::{get_extension_bytes, ExtensionType, VAULT_TLV_START};

pub struct PausableSubscriptions {
    pub paused: bool,
}

pub fn get_state(vault_data: &[u8]) -> Option<PausableSubscriptions> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(
        &vault_data[VAULT_TLV_START..],
        ExtensionType::PausableSubscriptions,
    )?;
    if bytes.is_empty() {
        return None;
    }
    Some(PausableSubscriptions {
        paused: bytes[0] != 0,
    })
}

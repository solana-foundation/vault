use borsh::BorshDeserialize;

use crate::FeeType;

use super::{get_extension_bytes, ExtensionType, VAULT_TLV_START};

pub fn get_deposit_fee(vault_data: &[u8]) -> Option<FeeType> {
    get_fee(vault_data, ExtensionType::DepositFee)
}

pub fn get_withdrawal_fee(vault_data: &[u8]) -> Option<FeeType> {
    get_fee(vault_data, ExtensionType::WithdrawalFee)
}

fn get_fee(vault_data: &[u8], ext_type: ExtensionType) -> Option<FeeType> {
    if vault_data.len() <= VAULT_TLV_START {
        return None;
    }
    let bytes = get_extension_bytes(&vault_data[VAULT_TLV_START..], ext_type)?;
    FeeType::try_from_slice(bytes).ok()
}

use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType},
};

pub const TLV_START: usize = 8 + 241;

pub fn get_fee_extension(account_data: &[u8], ext_type: ExtensionType) -> Result<Option<FeeType>> {
    if account_data.len() <= TLV_START {
        return Ok(None);
    }
    let tlv_data = &account_data[TLV_START..];
    match extensions::get_extension_bytes(tlv_data, ext_type) {
        Some(bytes) => {
            let mut slice = bytes;
            let fee = FeeType::deserialize(&mut slice)
                .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;
            Ok(Some(fee))
        }
        None => Ok(None),
    }
}

pub fn get_deposit_fee(account_data: &[u8], amount: u64) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::DepositFee)? {
        Some(fee) => Ok(fee.get_fee(amount)?),
        None => Ok(0),
    }
}

pub fn calculate_deposit_fee_when_minting(account_data: &[u8], net_assets: u64) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::DepositFee)? {
        Some(fee) => fee.get_deposit_fee_when_minting(net_assets),
        None => Ok(0),
    }
}

pub fn get_withdrawal_fee(account_data: &[u8], amount: u64) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
        Some(fee) => fee.get_fee(amount),
        None => Ok(0),
    }
}

pub fn calculate_withdraw_fee_when_redeeming(
    account_data: &[u8],
    gross_assets: u64,
) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
        Some(fee) => fee.get_withdraw_fee_when_redeeming(gross_assets),
        None => Ok(0),
    }
}

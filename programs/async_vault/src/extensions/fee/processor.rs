use anchor_lang::prelude::*;
use vault_common::{FeeType, VaultProgramError};

use crate::extensions::{read_vault_extension, ExtensionType};

/// TLV wrapper for a deposit fee. Serializes identically to the inner [`FeeType`],
/// and associates it with [`ExtensionType::DepositFee`] for generic TLV operations.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositFee(pub FeeType);

impl crate::extensions::VaultExtension for DepositFee {
    const DATA_SIZE: usize = 9;
    const EXTENSION_TYPE: ExtensionType = ExtensionType::DepositFee; // max Borsh size of FeeType (FixedAmount variant: 1 discriminant + 8 u64)
}

/// TLV wrapper for a withdrawal fee. Serializes identically to the inner [`FeeType`],
/// and associates it with [`ExtensionType::WithdrawalFee`] for generic TLV operations.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawalFee(pub FeeType);

impl crate::extensions::VaultExtension for WithdrawalFee {
    const DATA_SIZE: usize = 9;
    const EXTENSION_TYPE: ExtensionType = ExtensionType::WithdrawalFee; // max Borsh size of FeeType (FixedAmount variant: 1 discriminant + 8 u64)
}

pub fn get_deposit_fee(account_data: &[u8], amount: u64) -> Result<u64> {
    match read_vault_extension::<DepositFee>(account_data)? {
        Some(ext) => ext.0.get_fee(amount),
        None => Ok(0),
    }
}

pub fn get_withdrawal_fee(account_data: &[u8], amount: u64) -> Result<u64> {
    match read_vault_extension::<WithdrawalFee>(account_data)? {
        Some(ext) => ext.0.get_fee(amount),
        None => Ok(0),
    }
}

pub fn get_deposit_fee_and_net(account_data: &[u8], amount: u64) -> Result<(u64, u64)> {
    let fee = get_deposit_fee(account_data, amount)?;
    let net = amount
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok((fee, net))
}

pub fn get_withdrawal_fee_and_net(account_data: &[u8], amount: u64) -> Result<(u64, u64)> {
    let fee = get_withdrawal_fee(account_data, amount)?;
    let net = amount
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok((fee, net))
}

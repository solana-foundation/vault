use anchor_lang::prelude::*;
use vault_common::{FeeType, VaultProgramError};

use crate::extensions::{read_vault_extension, ExtensionType};

/// Pod-safe representation of a fee stored in TLV:
///   byte 0   — discriminant (0 = FixedAmount, 1 = Percentage)
///   bytes 1-8 — data (u64 LE for FixedAmount; u16 LE in bytes 1-2 for Percentage, rest zero)
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FeeData {
    pub discriminant: u8,
    pub data: [u8; 8],
}

impl FeeData {
    pub fn from_fee_type(fee_type: FeeType) -> Self {
        let mut data = [0u8; 8];
        match fee_type {
            FeeType::FixedAmount { amount } => {
                data.copy_from_slice(&amount.to_le_bytes());
                Self {
                    discriminant: 0,
                    data,
                }
            }
            FeeType::Percentage { bps } => {
                data[..2].copy_from_slice(&bps.to_le_bytes());
                Self {
                    discriminant: 1,
                    data,
                }
            }
        }
    }

    pub fn fee_type(&self) -> Result<FeeType> {
        match self.discriminant {
            0 => Ok(FeeType::FixedAmount {
                amount: u64::from_le_bytes(self.data),
            }),
            1 => Ok(FeeType::Percentage {
                bps: u16::from_le_bytes([self.data[0], self.data[1]]),
            }),
            _ => Err(crate::error::AsyncVaultError::InvalidExtensionData.into()),
        }
    }
}

/// Deposit fee extension. Transparent wrapper around [`FeeData`].
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(transparent)]
pub struct DepositFee(pub FeeData);

impl crate::extensions::VaultExtension for DepositFee {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::DepositFee;
}

impl DepositFee {
    pub fn from_fee_type(fee_type: FeeType) -> Self {
        Self(FeeData::from_fee_type(fee_type))
    }
}

impl std::ops::Deref for DepositFee {
    type Target = FeeData;

    fn deref(&self) -> &FeeData {
        &self.0
    }
}

/// Withdrawal fee extension. Transparent wrapper around [`FeeData`].
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(transparent)]
pub struct WithdrawalFee(pub FeeData);

impl crate::extensions::VaultExtension for WithdrawalFee {
    const EXTENSION_TYPE: ExtensionType = ExtensionType::WithdrawalFee;
}

impl WithdrawalFee {
    pub fn from_fee_type(fee_type: FeeType) -> Self {
        Self(FeeData::from_fee_type(fee_type))
    }
}

impl std::ops::Deref for WithdrawalFee {
    type Target = FeeData;

    fn deref(&self) -> &FeeData {
        &self.0
    }
}

pub fn get_deposit_fee(vault_info: &AccountInfo, amount: u64) -> Result<u64> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    match read_vault_extension::<DepositFee>(&data)? {
        Some(ext) => ext.fee_type()?.get_fee(amount),
        None => Ok(0),
    }
}

pub fn get_withdrawal_fee(vault_info: &AccountInfo, amount: u64) -> Result<u64> {
    let data = vault_info
        .data
        .try_borrow()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    match read_vault_extension::<WithdrawalFee>(&data)? {
        Some(ext) => ext.fee_type()?.get_fee(amount),
        None => Ok(0),
    }
}

pub fn get_deposit_fee_and_net(vault_info: &AccountInfo, amount: u64) -> Result<(u64, u64)> {
    let fee = get_deposit_fee(vault_info, amount)?;
    let net = amount
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok((fee, net))
}

pub fn get_withdrawal_fee_and_net(vault_info: &AccountInfo, amount: u64) -> Result<(u64, u64)> {
    let fee = get_withdrawal_fee(vault_info, amount)?;
    let net = amount
        .checked_sub(fee)
        .ok_or(VaultProgramError::ArithmeticError)?;
    Ok((fee, net))
}

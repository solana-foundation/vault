use anchor_lang::prelude::*;

use crate::{extensions::DepositHook, state::FeeType};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum VaultExtension {
    DepositFee(FeeType),
    WithdrawalFee(FeeType),
    DepositHook(DepositHook),
}

impl VaultExtension {
    pub fn as_deposit_fee(&self) -> Option<FeeType> {
        match self {
            VaultExtension::DepositFee(fee) => Some(*fee),
            _ => None,
        }
    }

    pub fn as_withdrawal_fee(&self) -> Option<FeeType> {
        match self {
            VaultExtension::WithdrawalFee(fee) => Some(*fee),
            _ => None,
        }
    }

    pub fn as_deposit_hook(&self) -> Option<DepositHook> {
        match self {
            VaultExtension::DepositHook(hook_program) => Some(*hook_program),
            _ => None,
        }
    }
}

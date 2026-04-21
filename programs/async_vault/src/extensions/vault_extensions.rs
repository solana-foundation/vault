use anchor_lang::prelude::*;

use crate::state::FeeType;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub enum VaultExtension {
    DepositFee(FeeType),
    WithdrawalFee(FeeType),
}

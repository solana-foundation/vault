use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{fee::WithdrawalFee, update_vault_extension, BasicExtensionAccounts},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateWithdrawalFeeArgs {
    pub new_withdrawal_fee: FeeType,
}

pub fn handler(ctx: Context<BasicExtensionAccounts>, args: UpdateWithdrawalFeeArgs) -> Result<()> {
    args.new_withdrawal_fee
        .validate()
        .map_err(AsyncVaultError::from)?;
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &WithdrawalFee::from_fee_type(args.new_withdrawal_fee),
    )
}

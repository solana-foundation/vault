use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::extensions::{fee::WithdrawalFee, BasicExtensionAccounts, update_vault_extension};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateWithdrawalFeeArgs {
    pub new_withdrawal_fee: FeeType,
}

pub fn handler(ctx: Context<BasicExtensionAccounts>, args: UpdateWithdrawalFeeArgs) -> Result<()> {
    args.new_withdrawal_fee.validate()?;
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &WithdrawalFee(args.new_withdrawal_fee),
    )
}

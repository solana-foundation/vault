use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::extensions::{fee::DepositFee, BasicExtensionAccounts, update_vault_extension};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateDepositFeeArgs {
    pub new_deposit_fee: FeeType,
}

pub fn handler(ctx: Context<BasicExtensionAccounts>, args: UpdateDepositFeeArgs) -> Result<()> {
    args.new_deposit_fee.validate()?;
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &DepositFee(args.new_deposit_fee),
    )
}

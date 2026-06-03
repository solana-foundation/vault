use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{fee::DepositFee, update_vault_extension, BasicExtensionAccounts},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateDepositFeeArgs {
    pub new_deposit_fee: FeeType,
}

pub fn handler(ctx: Context<BasicExtensionAccounts>, args: UpdateDepositFeeArgs) -> Result<()> {
    args.new_deposit_fee.validate().map_err(AsyncVaultError::from)?;
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &DepositFee::from_fee_type(args.new_deposit_fee),
    )
}

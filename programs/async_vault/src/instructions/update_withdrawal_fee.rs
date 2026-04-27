use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, TLV_START},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateWithdrawalFeeArgs {
    pub new_withdrawal_fee: FeeType,
}

#[derive(Accounts)]
pub struct UpdateWithdrawalFee<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<UpdateWithdrawalFee>, args: UpdateWithdrawalFeeArgs) -> Result<()> {
    args.new_withdrawal_fee.validate()?;

    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info.data.borrow_mut();
    let tlv_start = TLV_START;
    let tlv_data = &mut data[tlv_start..];

    let serialized = args
        .new_withdrawal_fee
        .try_to_vec()
        .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;

    extensions::update_extension(tlv_data, ExtensionType::WithdrawalFee, &serialized)?;

    Ok(())
}

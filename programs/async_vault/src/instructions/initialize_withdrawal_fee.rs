use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, FEE_TLV_SIZE, TLV_START},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitWithdrawalFeeArgs {
    pub withdrawal_fee: FeeType,
}

#[derive(Accounts)]
pub struct InitWithdrawalFee<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + FEE_TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitWithdrawalFee>, args: InitWithdrawalFeeArgs) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;
    args.withdrawal_fee.validate()?;

    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info.data.borrow_mut();
    let tlv_start = TLV_START;
    let tlv_data = &mut data[tlv_start..];

    require!(
        !extensions::has_extension(tlv_data, ExtensionType::WithdrawalFee),
        AsyncVaultError::ExtensionAlreadyInitialized
    );

    let write_offset = extensions::tlv_used_len(tlv_data);
    let serialized = args
        .withdrawal_fee
        .try_to_vec()
        .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;

    extensions::write_extension(
        tlv_data,
        write_offset,
        ExtensionType::WithdrawalFee,
        &serialized,
    )?;

    Ok(())
}

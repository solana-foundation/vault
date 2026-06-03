use anchor_lang::prelude::*;
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{fee::DepositFee, init_vault_extension, VaultExtension},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitDepositFeeArgs {
    pub deposit_fee: FeeType,
}

#[derive(Accounts)]
pub struct InitDepositFee<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + DepositFee::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitDepositFee>, args: InitDepositFeeArgs) -> Result<()> {
    args.deposit_fee.validate().map_err(AsyncVaultError::from)?;
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &DepositFee::from_fee_type(args.deposit_fee),
    )
}

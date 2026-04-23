use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use vault_common::FeeType;

use crate::{
    error::VaultProgramError,
    extensions::VaultExtension,
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateDepositFeesArgs {
    new_deposit_fee: FeeType,
}

#[derive(Accounts)]
pub struct UpdateDepositFees<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler<'info>(ctx: Context<UpdateDepositFees>, args: UpdateDepositFeesArgs) -> Result<()> {
    match ctx.accounts.vault.deposit_fee_type() {
        Some((index, _)) => {
            args.new_deposit_fee.validate()?;

            ctx.accounts.vault.extensions[index] = VaultExtension::DepositFee(args.new_deposit_fee);

            Ok(())
        }
        None => Err(VaultProgramError::UninitializedExtension.into()),
    }
}

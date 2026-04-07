use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    extensions::VaultExtension,
    state::{FeeType, Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateWithdrawalFeesArgs {
    new_withdrawal_fee: FeeType,
}

#[derive(Accounts)]
pub struct UpdateWithdrawalFees<'info> {
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

pub fn handler<'info>(
    ctx: Context<UpdateWithdrawalFees>,
    args: UpdateWithdrawalFeesArgs,
) -> Result<()> {
    match ctx.accounts.vault.withdrawal_fee_type() {
        Some((index, _)) => {
            args.new_withdrawal_fee.validate()?;

            ctx.accounts.vault.extensions[index] =
                VaultExtension::WithdrawalFee(args.new_withdrawal_fee);

            Ok(())
        }
        None => Err(VaultProgramError::UninitializedExtension.into()),
    }
}

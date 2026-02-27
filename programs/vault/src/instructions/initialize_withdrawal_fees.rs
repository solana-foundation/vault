use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    state::{FeeType, VaultConfig, VaultExtension, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitWithdrawalFeesArgs {
    withdrawal_fee: FeeType,
}

#[derive(Accounts)]
pub struct InitWithdrawalFees<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ VaultProgramError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultConfig>,
}

pub fn handler<'info>(
    ctx: Context<InitWithdrawalFees>,
    args: InitWithdrawalFeesArgs,
) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let is_withdrawal_fee_present = ctx.accounts.vault.withdrawal_fee_type().is_some();

    if is_withdrawal_fee_present {
        return Err(VaultProgramError::ExtensionAlreadyInitialized.into());
    }

    args.withdrawal_fee.validate()?;

    ctx.accounts
        .vault
        .extensions
        .push(VaultExtension::WithdrawalFee(args.withdrawal_fee));

    Ok(())
}

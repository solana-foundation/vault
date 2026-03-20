use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    extensions::VaultExtension,
    state::{FeeType, VaultConfig, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitDepositFeesArgs {
    deposit_fee: FeeType,
}

#[derive(Accounts)]
pub struct InitDepositFees<'info> {
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

pub fn handler<'info>(ctx: Context<InitDepositFees>, args: InitDepositFeesArgs) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let is_deposit_fee_present = ctx.accounts.vault.deposit_fee_type().is_some();

    if is_deposit_fee_present {
        return Err(VaultProgramError::ExtensionAlreadyInitialized.into());
    }

    args.deposit_fee.validate()?;

    ctx.accounts
        .vault
        .extensions
        .push(VaultExtension::DepositFee(args.deposit_fee));

    Ok(())
}

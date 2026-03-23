use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    extensions::{VaultExtension, WithdrawHook},
    state::{VaultConfig, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct InitializeWithdrawHook<'info> {
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

pub fn handler<'info>(ctx: Context<InitializeWithdrawHook>, hook_program: Pubkey) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let is_withdraw_hook_present = ctx.accounts.vault.withdraw_hook_type().is_some();

    if is_withdraw_hook_present {
        return Err(VaultProgramError::ExtensionAlreadyInitialized.into());
    }

    ctx.accounts
        .vault
        .extensions
        .push(VaultExtension::WithdrawHook(WithdrawHook {
            hook_program_id: hook_program,
            authority: ctx.accounts.authority.key(),
        }));

    Ok(())
}

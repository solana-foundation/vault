use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultProgramError,
    extensions::{DepositHook, VaultExtension},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct InitializeDepositHook<'info> {
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

pub fn handler<'info>(ctx: Context<InitializeDepositHook>, hook_program: Pubkey) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let is_deposit_hook_present = ctx.accounts.vault.deposit_hook_type().is_some();

    if is_deposit_hook_present {
        return Err(VaultProgramError::ExtensionAlreadyInitialized.into());
    }

    ctx.accounts
        .vault
        .extensions
        .push(VaultExtension::DepositHook(DepositHook {
            hook_program_id: hook_program,
            authority: ctx.accounts.authority.key(),
        }));

    Ok(())
}

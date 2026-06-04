use anchor_lang::{prelude::*, solana_program::program_option::COption};
use anchor_spl::token_interface::Mint;

use crate::{error::AsyncVaultError, state::Vault};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = share_mint @ AsyncVaultError::InvalidShareMint,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    require!(
        ctx.accounts.share_mint.mint_authority == COption::Some(ctx.accounts.vault.key()),
        AsyncVaultError::InvalidShareMint
    );
    require!(
        ctx.accounts.share_mint.supply == 0,
        AsyncVaultError::ShareMintSupplyShouldBeZero
    );

    ctx.accounts.vault.initialized = true;

    Ok(())
}

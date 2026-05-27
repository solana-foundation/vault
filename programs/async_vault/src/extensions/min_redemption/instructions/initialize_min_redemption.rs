use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{init_vault_extension, min_redemption::MinRedemption, VaultExtension},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitMinRedemptionArgs {
    pub threshold: u64,
}

/// Accounts for initializing the MinRedemption extension.
#[derive(Accounts)]
pub struct InitMinRedemption<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + MinRedemption::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

/// Adds the MinRedemption TLV extension to the vault.
pub fn handler(ctx: Context<InitMinRedemption>, args: InitMinRedemptionArgs) -> Result<()> {
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &MinRedemption {
            threshold: args.threshold,
        },
    )
}

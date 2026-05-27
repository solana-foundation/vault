use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{init_vault_extension, min_subscription::MinSubscription, VaultExtension},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitMinSubscriptionArgs {
    pub threshold: u64,
}

/// Accounts for initializing the MinSubscription extension.
#[derive(Accounts)]
pub struct InitMinSubscription<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + MinSubscription::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

/// Adds the MinSubscription TLV extension to the vault.
pub fn handler(ctx: Context<InitMinSubscription>, args: InitMinSubscriptionArgs) -> Result<()> {
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &MinSubscription {
            threshold: args.threshold,
        },
    )
}

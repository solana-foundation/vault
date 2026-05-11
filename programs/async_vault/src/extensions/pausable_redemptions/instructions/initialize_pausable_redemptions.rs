use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{init_vault_extension, pausable_redemptions::PausableRedemption, VaultExtension},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitPausableRedemptionsArgs {
    pub paused: bool,
}

#[derive(Accounts)]
pub struct InitPausableRedemptions<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + PausableRedemption::TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitPausableRedemptions>,
    args: InitPausableRedemptionsArgs,
) -> Result<()> {
    init_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.vault,
        &PausableRedemption {
            paused: args.paused as u8,
        },
    )
}

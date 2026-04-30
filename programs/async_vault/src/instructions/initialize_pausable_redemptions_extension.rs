use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, PausableExtension, PAUSABLE_TLV_SIZE, TLV_START},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct InitPausableRedemption<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + PAUSABLE_TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitPausableRedemption>) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info.data.borrow_mut();
    let tlv_data = &mut data[TLV_START..];

    require!(
        !extensions::has_extension(tlv_data, ExtensionType::PausableRedemptionsExtension),
        AsyncVaultError::ExtensionAlreadyInitialized
    );

    let write_offset = extensions::tlv_used_len(tlv_data);
    let extension = PausableExtension { paused: false };
    let serialized = extension
        .try_to_vec()
        .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;

    extensions::write_extension(
        tlv_data,
        write_offset,
        ExtensionType::PausableRedemptionsExtension,
        &serialized,
    )?;

    Ok(())
}

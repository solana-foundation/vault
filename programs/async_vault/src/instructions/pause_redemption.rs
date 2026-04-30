use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType, PausableExtension, TLV_START},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(Accounts)]
pub struct PauseRedemption<'info> {
    pub authority: Signer<'info>,

    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<PauseRedemption>, paused: bool) -> Result<()> {
    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info.data.borrow_mut();
    let tlv_data = &mut data[TLV_START..];

    let extension = PausableExtension { paused };
    let serialized = extension
        .try_to_vec()
        .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;

    extensions::update_extension(
        tlv_data,
        ExtensionType::PausableRedemptionsExtension,
        &serialized,
    )?;

    Ok(())
}

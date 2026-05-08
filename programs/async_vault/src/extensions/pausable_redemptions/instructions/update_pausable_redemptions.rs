use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{self, pausable_redemptions::PausableRedemption, ExtensionType, TLV_START},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePausableRedemptionsArgs {
    pub paused: bool,
}

#[derive(Accounts)]
pub struct UpdatePausableRedemptions<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(
    ctx: Context<UpdatePausableRedemptions>,
    args: UpdatePausableRedemptionsArgs,
) -> Result<()> {
    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let tlv_data = &mut data[TLV_START..];

    let serialized = PausableRedemption {
        paused: args.paused,
    }
    .try_to_vec()
    .map_err(|_| AsyncVaultError::InvalidExtensionData)?;

    extensions::update_extension(tlv_data, ExtensionType::PausableRedemptions, &serialized)?;
    Ok(())
}

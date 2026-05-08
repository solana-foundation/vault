use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{self, pausable_subscriptions::PausableSubscription, ExtensionType, TLV_START},
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePausableSubscriptionsArgs {
    pub paused: bool,
}

#[derive(Accounts)]
pub struct UpdatePausableSubscriptions<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(
    ctx: Context<UpdatePausableSubscriptions>,
    args: UpdatePausableSubscriptionsArgs,
) -> Result<()> {
    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let tlv_data = &mut data[TLV_START..];

    let serialized = PausableSubscription {
        paused: args.paused,
    }
    .try_to_vec()
    .map_err(|_| AsyncVaultError::InvalidExtensionData)?;

    extensions::update_extension(tlv_data, ExtensionType::PausableSubscriptions, &serialized)?;
    Ok(())
}

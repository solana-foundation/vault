use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::AsyncVaultError,
    extensions::{self, pausable_subscriptions::PausableSubscription, ExtensionType, TLV_START},
    state::{Vault, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePausableSubscriptionsArgs {
    pub paused: bool,
}

#[derive(Accounts)]
pub struct UpdatePausableSubscriptions<'info> {
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

pub fn handler(
    ctx: Context<UpdatePausableSubscriptions>,
    args: UpdatePausableSubscriptionsArgs,
) -> Result<()> {
    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info.data.borrow_mut();
    let tlv_data = &mut data[TLV_START..];

    let serialized = PausableSubscription {
        paused: args.paused,
    }
    .try_to_vec()
    .map_err(|_| AsyncVaultError::InvalidExtensionData)?;

    extensions::update_extension(tlv_data, ExtensionType::PausableSubscriptions, &serialized)?;
    Ok(())
}

use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    extensions::{
        self, pausable_subscriptions::PausableSubscription, ExtensionType,
        PAUSABLE_SUBSCRIPTIONS_TLV_SIZE, TLV_START,
    },
    state::Vault,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitPausableSubscriptionsArgs {
    pub paused: bool,
}

#[derive(Accounts)]
pub struct InitPausableSubscriptions<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        realloc = vault.to_account_info().data_len() + PAUSABLE_SUBSCRIPTIONS_TLV_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitPausableSubscriptions>,
    args: InitPausableSubscriptionsArgs,
) -> Result<()> {
    ctx.accounts.vault.assert_uninitialized()?;

    let vault_info = ctx.accounts.vault.to_account_info();
    let mut data = vault_info
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let tlv_data = &mut data[TLV_START..];

    require!(
        !extensions::has_extension(tlv_data, ExtensionType::PausableSubscriptions),
        AsyncVaultError::ExtensionAlreadyInitialized
    );

    let write_offset = extensions::tlv_used_len(tlv_data);
    let serialized = PausableSubscription {
        paused: args.paused,
    }
    .try_to_vec()
    .map_err(|_| AsyncVaultError::InvalidExtensionData)?;

    extensions::write_extension(
        tlv_data,
        write_offset,
        ExtensionType::PausableSubscriptions,
        &serialized,
    )?;

    Ok(())
}

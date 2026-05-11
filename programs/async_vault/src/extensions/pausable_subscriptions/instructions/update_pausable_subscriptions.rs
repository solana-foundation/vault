use anchor_lang::prelude::*;

use crate::extensions::{
    pausable_subscriptions::PausableSubscription, update_vault_extension, BasicExtensionAccounts,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePausableSubscriptionsArgs {
    pub paused: bool,
}

pub fn handler(
    ctx: Context<BasicExtensionAccounts>,
    args: UpdatePausableSubscriptionsArgs,
) -> Result<()> {
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &PausableSubscription {
            paused: args.paused as u8,
        },
    )
}

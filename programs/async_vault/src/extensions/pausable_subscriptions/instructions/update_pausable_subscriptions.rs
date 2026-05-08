use anchor_lang::prelude::*;

use crate::extensions::{
    pausable_subscriptions::PausableSubscription, BasicExtensionAccounts, update_vault_extension,
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
        &PausableSubscription { paused: args.paused },
    )
}

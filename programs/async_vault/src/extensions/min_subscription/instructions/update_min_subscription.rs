use anchor_lang::prelude::*;

use crate::extensions::{
    min_subscription::MinSubscription, update_vault_extension, BasicExtensionAccounts,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateMinSubscriptionArgs {
    pub threshold: u64,
}

/// Updates the threshold of an existing MinSubscription extension.
pub fn handler(
    ctx: Context<BasicExtensionAccounts>,
    args: UpdateMinSubscriptionArgs,
) -> Result<()> {
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &MinSubscription {
            threshold: args.threshold,
        },
    )
}

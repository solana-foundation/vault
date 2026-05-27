use anchor_lang::prelude::*;

use crate::extensions::{
    min_redemption::MinRedemption, update_vault_extension, BasicExtensionAccounts,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateMinRedemptionArgs {
    pub threshold: u64,
}

/// Updates the threshold of an existing MinRedemption extension.
pub fn handler(ctx: Context<BasicExtensionAccounts>, args: UpdateMinRedemptionArgs) -> Result<()> {
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &MinRedemption {
            threshold: args.threshold,
        },
    )
}

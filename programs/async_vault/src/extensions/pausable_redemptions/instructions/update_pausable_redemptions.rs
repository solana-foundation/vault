use anchor_lang::prelude::*;

use crate::extensions::{
    pausable_redemptions::PausableRedemption, update_vault_extension, BasicExtensionAccounts,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePausableRedemptionsArgs {
    pub paused: bool,
}

pub fn handler(
    ctx: Context<BasicExtensionAccounts>,
    args: UpdatePausableRedemptionsArgs,
) -> Result<()> {
    update_vault_extension(
        &ctx.accounts.vault.to_account_info(),
        &PausableRedemption {
            paused: args.paused as u8,
        },
    )
}

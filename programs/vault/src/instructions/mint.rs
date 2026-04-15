use anchor_lang::prelude::*;

use crate::{
    instructions::{vault_common, VaultCommon},
    state::{VaultAction, VaultActionParams},
};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    shares: u64,
    max_assets: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        VaultAction::Mint,
        VaultActionParams {
            amount: shares,
            threshold_amount: max_assets,
        },
    )
}

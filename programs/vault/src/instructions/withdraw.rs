use anchor_lang::prelude::*;

use crate::{
    instructions::{vault_common, VaultCommon},
    state::{VaultAction, VaultActionParams},
};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    assets: u64,
    max_shares: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        VaultAction::Withdraw,
        VaultActionParams {
            amount: assets,
            threshold_amount: max_shares,
        },
    )
}

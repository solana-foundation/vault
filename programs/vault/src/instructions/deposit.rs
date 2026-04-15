use anchor_lang::prelude::*;

use crate::{
    instructions::vault_common,
    state::{VaultAction, VaultActionParams},
};

use super::VaultCommon;

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    assets: u64,
    min_shares: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        VaultAction::Deposit,
        VaultActionParams {
            amount: assets,
            threshold_amount: min_shares,
        },
    )
}

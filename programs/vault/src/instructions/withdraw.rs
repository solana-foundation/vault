use anchor_lang::prelude::*;

use crate::{
    instructions::{vault_common, VaultCommon},
    state::{SwapKind, SwapParams},
};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    assets: u64,
    max_shares: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        SwapKind::Withdraw,
        SwapParams {
            amount: assets,
            threshold_amount: max_shares,
        },
    )
}

use anchor_lang::prelude::*;

use crate::{
    instructions::{vault_common, VaultCommon},
    state::{SwapKind, SwapParams},
};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    shares: u64,
    min_assets: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        SwapKind::Redeem,
        SwapParams {
            amount: shares,
            threshold_amount: min_assets,
        },
    )
}

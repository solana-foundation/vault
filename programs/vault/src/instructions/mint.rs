use anchor_lang::prelude::*;

use crate::{
    instructions::{vault_common, VaultCommon},
    state::{SwapKind, SwapParams},
};

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    shares: u64,
    max_assets: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        SwapKind::Mint,
        SwapParams {
            amount: shares,
            threshold_amount: max_assets,
        },
    )
}

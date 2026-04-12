use anchor_lang::prelude::*;

use crate::{
    instructions::vault_common,
    state::{SwapKind, SwapParams},
};

use super::VaultCommon;

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, VaultCommon<'info>>,
    assets: u64,
    min_shares: u64,
) -> Result<()> {
    vault_common::handler(
        ctx,
        SwapKind::Deposit,
        SwapParams {
            amount: assets,
            threshold_amount: min_shares,
        },
    )
}

use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program::{get_return_data, invoke, invoke_signed},
    },
};
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{
        self, approve_checked, mint_to, ApproveChecked, Mint, MintTo, TokenAccount, TokenInterface,
        TransferChecked,
    },
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use crate::{
    error::VaultProgramError,
    extensions::{
        create_deposit_hook_ix, get_deposit_hook_extra_account_metas_address, DepositHook,
        DepositHookInstruction,
    },
    instructions::vault_common,
    state::{
        Rounding, SwapKind, SwapParams, Vault, MAX_BPS, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
    },
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

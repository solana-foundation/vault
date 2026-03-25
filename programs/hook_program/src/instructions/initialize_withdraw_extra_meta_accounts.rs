use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use vault::state::WITHDRAW_ACCOUNT_METAS_SEED;

use crate::state::{
    WithdrawHookInstruction, EXTRA_ACCOUNT_METAS_SEED, VAULT_ASSOCIATED_PROTOCOLS_SEED,
    VAULT_PROGRAM_ID, VAULT_PROTOCOL_DEPOSIT_SEED, VAULT_SEED,
};

#[derive(Accounts)]
pub struct InitializeWithdrawExtraMetaAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub share_mint_address: InterfaceAccount<'info, Mint>,

    /// CHECK: extra metas, it's checked by seeds
    #[account(
        init,
        payer = payer,
        space = get_extra_metas_size(),
        seeds = [EXTRA_ACCOUNT_METAS_SEED,WITHDRAW_ACCOUNT_METAS_SEED, share_mint_address.key().as_ref()],
        bump
    )]
    pub extra_metas: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl InitializeWithdrawExtraMetaAccounts<'_> {
    pub fn create_extra_account_meta(&mut self) -> Result<()> {
        let extra_metas_account = &self.extra_metas;
        let metas = get_extra_metas();
        let mut data = extra_metas_account.try_borrow_mut_data()?;
        ExtraAccountMetaList::init::<WithdrawHookInstruction>(&mut data, &metas?)?;
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<InitializeWithdrawExtraMetaAccounts>) -> Result<()> {
    ctx.accounts.create_extra_account_meta()
}

fn get_extra_metas() -> Result<Vec<ExtraAccountMeta>> {
    let associated_protocols_meta = ExtraAccountMeta::new_with_seeds(
        &[
            Seed::Literal {
                bytes: VAULT_ASSOCIATED_PROTOCOLS_SEED.to_vec(),
            },
            Seed::AccountKey { index: 1 }, // share mint
        ],
        false,
        false,
    )?;
    let vault_state_meta = ExtraAccountMeta::new_external_pda_with_seeds(
        3, // external protocol token program index
        &[
            Seed::Literal {
                bytes: VAULT_SEED.to_vec(),
            },
            Seed::AccountKey { index: 1 }, // share mint
        ],
        false,
        true,
    )?;

    Ok([associated_protocols_meta, vault_state_meta].to_vec())
}

fn get_extra_metas_size() -> usize {
    ExtraAccountMetaList::size_of(2).unwrap()
}

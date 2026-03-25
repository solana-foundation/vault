use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};

use crate::state::{
    DepositHookInstruction, DEPOSIT_ACCOUNT_METAS_SEED, EXTRA_ACCOUNT_METAS_SEED,
    VAULT_ASSOCIATED_PROTOCOLS_SEED, VAULT_SEED,
};

#[derive(Accounts)]
pub struct InitializeDepositExtraMetaAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub share_mint_address: InterfaceAccount<'info, Mint>,

    /// CHECK: extra metas, it's checked by seeds
    #[account(
        init,
        payer = payer,
        space = get_extra_metas_size(),
        seeds = [EXTRA_ACCOUNT_METAS_SEED,DEPOSIT_ACCOUNT_METAS_SEED, share_mint_address.key().as_ref()],
        bump
    )]
    pub extra_metas: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl InitializeDepositExtraMetaAccounts<'_> {
    pub fn create_extra_account_meta(&mut self) -> Result<()> {
        let extra_metas_account = &self.extra_metas;
        let metas = get_extra_metas();
        let mut data = extra_metas_account.try_borrow_mut_data()?;
        ExtraAccountMetaList::init::<DepositHookInstruction>(&mut data, &metas?)?;
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<InitializeDepositExtraMetaAccounts>) -> Result<()> {
    ctx.accounts.create_extra_account_meta()
}

fn get_extra_metas() -> Result<Vec<ExtraAccountMeta>> {
    // Must be first: lands at index 6 = `associated_protocols_info` in ExecuteDepositHook
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
    // Must be second: lands at index 7 = remaining_accounts[0] used by invoke_deposit
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

use anchor_lang::{
    prelude::*,
    solana_program::{
        program::set_return_data,
        sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
    },
};

use crate::{
    errors::HookProgramError,
    state::{NavReturnData, UPDATE_NAV_DISCRIMINATOR, VAULT_NAV_DATA},
};

#[derive(Accounts)]
pub struct GetNavData<'info> {
    /// CHECK: This is vault
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_NAV_DATA, vault.key().as_ref()],
        bump
    )]
    pub nav_return_data: Account<'info, NavReturnData>,

    /// CHECK: Instructions sysvar for transaction introspection
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: AccountInfo<'info>,
}

pub fn handler<'info>(ctx: Context<GetNavData>) -> Result<()> {
    let ix_sysvar = &ctx.accounts.instructions;
    let vault_key = ctx.accounts.vault.key();
    let current_index = load_current_index_checked(ix_sysvar)? as usize;

    let update_nav_found = (0..current_index).any(|i| {
        load_instruction_at_checked(i, ix_sysvar)
            .map(|ix| {
                ix.program_id == crate::ID
                    && ix.data.starts_with(&UPDATE_NAV_DISCRIMINATOR)
                    && ix.accounts.get(1).map(|a| a.pubkey) == Some(vault_key)
            })
            .unwrap_or(false)
    });

    require!(
        update_nav_found,
        HookProgramError::UpdateNavNotCalledBeforeGetNav
    );

    let data = ctx
        .accounts
        .nav_return_data
        .to_account_info()
        .try_borrow_data()?
        .to_vec();
    set_return_data(&data);
    Ok(())
}

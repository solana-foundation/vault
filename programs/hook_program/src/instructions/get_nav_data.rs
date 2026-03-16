use anchor_lang::{prelude::*, solana_program::program::set_return_data};

use crate::state::{NavReturnData, VAULT_NAV_DATA};

#[derive(Accounts)]
pub struct GetNavData<'info> {
    /// CHECK: This is vault
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_NAV_DATA, vault.key().as_ref()],
        bump
    )]
    pub nav_return_data: Account<'info, NavReturnData>,
}

pub fn handler<'info>(ctx: Context<GetNavData>) -> Result<()> {
    let data = ctx.accounts.nav_return_data.try_to_vec()?;
    set_return_data(&data);
    Ok(())
}

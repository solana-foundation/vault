use anchor_lang::prelude::*;

use crate::{
    error::AsyncVaultError,
    state::{Request, Vault},
};

#[derive(Accounts)]
pub struct SetOperator<'info> {
    pub authority: Signer<'info>,

    pub operator: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub request: Account<'info, Request>,
}

pub fn handler(ctx: Context<SetOperator>) -> Result<()> {
    ctx.accounts.request.operator = Some(ctx.accounts.operator.key());
    Ok(())
}

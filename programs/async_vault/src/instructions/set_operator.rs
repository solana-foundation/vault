use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::Request};

#[derive(Accounts)]
pub struct SetOperator<'info> {
    pub user: Signer<'info>,

    pub operator: Signer<'info>,

    #[account(
        mut,
        constraint = user.key() == request.owner @ AsyncVaultError::UnauthorizedSigner
    )]
    pub request: Account<'info, Request>,
}

pub fn handler(ctx: Context<SetOperator>) -> Result<()> {
    ctx.accounts.request.operator = Some(ctx.accounts.operator.key());
    Ok(())
}

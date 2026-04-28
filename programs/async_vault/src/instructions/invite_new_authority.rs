use anchor_lang::prelude::*;

use crate::{error::AsyncVaultError, state::Vault};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InviteNewAuthorityArgs {
    pub new_authority: Pubkey,
}

#[derive(Accounts)]
pub struct InviteNewAuthority<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == vault.authority @ AsyncVaultError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<InviteNewAuthority>, args: InviteNewAuthorityArgs) -> Result<()> {
    ctx.accounts.vault.pending_authority = Some(args.new_authority);
    Ok(())
}

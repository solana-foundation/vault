use anchor_lang::{prelude::*, solana_program::program::set_return_data};
use anchor_spl::token_interface::Mint;

use crate::state::{get_nav, VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS_SEED};

#[derive(Accounts)]
pub struct GetNav<'info> {
    pub share_mint: InterfaceAccount<'info, Mint>,
    #[account(
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub associated_protocols_info: Account<'info, VaultAssociatedProtocols>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, GetNav<'info>>) -> Result<()> {
    let data = get_nav(
        &ctx.accounts.associated_protocols_info.protocols,
        &ctx.accounts.share_mint.key(),
        ctx.accounts.share_mint.supply,
        ctx.accounts.share_mint.decimals,
        ctx.remaining_accounts,
        ctx.program_id,
    )?;
    let bytes = data.try_to_vec()?;
    set_return_data(&bytes);
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[derive(Accounts)]
pub struct DepositHook<'info> {
    pub share_mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DepositHook>) -> Result<()> {
    msg!("Executing Hook");
    Ok(())
}

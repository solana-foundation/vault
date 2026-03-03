use anchor_lang::prelude::*;
use anchor_spl::{
    token::spl_token,
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{self, mint_to, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
pub struct DepositHook<'info> {
    pub share_mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DepositHook>) -> Result<()> {
    msg!("Executing Hook");
    Ok(())
}

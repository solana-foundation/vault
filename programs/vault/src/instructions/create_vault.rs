use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{set_authority, spl_token_2022::instruction::AuthorityType, SetAuthority},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    error::VaultProgramError,
    state::{Vault, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct VaultArgs {
    authority: Pubkey,
    initial_price: u64,
    vault_asset_cap: Option<u64>,
    fee_recipient: Pubkey,
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        token::authority = vault,
        token::mint = asset_mint,
        token::token_program = asset_token_program,
        payer = payer,
        seeds = [RESERVE_CONFIG_SEED, share_mint.key().as_ref()],
        bump,
    )]
    pub reserve: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        space = 8 + Vault::INIT_SPACE,
        payer = payer,
        seeds = [VAULT_CONFIG_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub share_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateVault<'info> {
    pub fn set_new_authority(&mut self, new_authority: Pubkey) -> Result<()> {
        let set_authority_cpi_accounts = SetAuthority {
            current_authority: self.mint_authority.to_account_info(),
            account_or_mint: self.share_mint.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.share_token_program.to_account_info(),
            set_authority_cpi_accounts,
        );
        set_authority(cpi_ctx, AuthorityType::MintTokens, Some(new_authority))
    }
}

pub fn handler<'info>(ctx: Context<CreateVault>, args: VaultArgs) -> Result<()> {
    require!(
        args.initial_price != 0,
        VaultProgramError::InvalidInitialPrice
    );
    ctx.accounts.set_new_authority(ctx.accounts.vault.key())?;
    ctx.accounts.vault.set_inner(Vault {
        asset_mint_address: ctx.accounts.asset_mint.key(),
        share_mint_address: ctx.accounts.share_mint.key(),
        vault_token_account: ctx.accounts.reserve.key(),
        authority: args.authority,
        initial_price: args.initial_price,
        paused: true,
        initialized: false,
        vault_asset_cap: args.vault_asset_cap.unwrap_or(0),
        fee_recipient: args.fee_recipient,
        extensions: vec![],
        reserve_bump: ctx.bumps.reserve,
        bump: ctx.bumps.vault,
    });
    Ok(())
}

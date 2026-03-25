use anchor_lang::{
    prelude::*,
    solana_program::program::{invoke, set_return_data},
};
use anchor_spl::token_interface::Mint;

use crate::{
    errors::HookProgramError,
    state::{
        get_nav, protocol_deposit, validate_protocols, VaultAssociatedProtocols,
        VAULT_ASSOCIATED_PROTOCOLS_SEED,
    },
};

#[derive(Accounts)]
pub struct ExecuteDepositHook<'info> {
    // This should be the vault authority
    pub signer: Signer<'info>,
    pub share_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: This is the extra metas
    pub extra_metas: AccountInfo<'info>,
    /// CHECK: This is downstream protocol
    pub protocol: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is the downstream protocol vault
    pub vault: UncheckedAccount<'info>,
    #[account(
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub associated_protocols_info: Account<'info, VaultAssociatedProtocols>,
}

impl<'info> ExecuteDepositHook<'info> {
    pub fn invoke_deposit(&self, additional_accounts: &[AccountInfo<'info>]) -> Result<()> {
        let downstream_vault = additional_accounts
            .first()
            .ok_or(error!(HookProgramError::InvalidAccountData))?;

        let instruction = protocol_deposit(
            &self.protocol.key(),
            self.signer.key,
            &self.share_mint.key(),
            &downstream_vault.key(),
            &self.system_program.key(),
        );

        let mut cpi_account_infos = vec![
            self.signer.to_account_info(),
            self.share_mint.to_account_info(),
            downstream_vault.clone(),
            self.system_program.to_account_info(),
        ];
        cpi_account_infos.extend_from_slice(additional_accounts);

        invoke(&instruction, &cpi_account_infos)?;
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, ExecuteDepositHook<'info>>) -> Result<()> {
    validate_protocols(
        &ctx.accounts.associated_protocols_info.protocols,
        ctx.accounts.protocol.key,
    )?;
    ctx.accounts.invoke_deposit(ctx.remaining_accounts)?;
    let total = get_nav(
        &ctx.accounts.associated_protocols_info.protocols,
        &ctx.accounts.share_mint.key(),
        ctx.remaining_accounts,
        ctx.program_id,
    )?;
    let data = total.try_to_vec()?;
    set_return_data(&data);
    Ok(())
}

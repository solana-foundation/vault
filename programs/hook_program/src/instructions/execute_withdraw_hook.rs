use anchor_lang::{prelude::*, solana_program::program::invoke};
use anchor_spl::token_interface::Mint;

use crate::{errors::HookProgramError, state::protocol_deposit};

#[derive(Accounts)]
pub struct ExecuteWithdrawHook<'info> {
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
}

impl<'info> ExecuteWithdrawHook<'info> {
    pub fn invoke_withdraw(&self, additional_accounts: &[AccountInfo<'info>]) -> Result<()> {
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

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, ExecuteWithdrawHook<'info>>) -> Result<()> {
    ctx.accounts.invoke_withdraw(ctx.remaining_accounts)?;
    Ok(())
}

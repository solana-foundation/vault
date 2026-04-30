use anchor_lang::{
    prelude::*,
    solana_program::program::{invoke, set_return_data},
};
use anchor_spl::token_interface::Mint;

use crate::{
    errors::HookProgramError,
    state::{
        get_total_assets, protocol_withdraw, validate_protocols, VaultAssociatedProtocols,
        VAULT_ASSOCIATED_PROTOCOLS_SEED,
    },
};

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
    #[account(
        seeds = [VAULT_ASSOCIATED_PROTOCOLS_SEED, share_mint.key().as_ref()],
        bump
    )]
    pub associated_protocols_info: Account<'info, VaultAssociatedProtocols>,
}

impl<'info> ExecuteWithdrawHook<'info> {
    pub fn invoke_withdraw(
        &self,
        additional_accounts: &[AccountInfo<'info>],
        amount: u64,
    ) -> Result<()> {
        let downstream_vault = additional_accounts
            .first()
            .ok_or(HookProgramError::InvalidAccountData)?;

        let instruction = protocol_withdraw(
            &self.protocol.key(),
            self.signer.key,
            &self.share_mint.key(),
            &downstream_vault.key(),
            &self.system_program.key(),
            amount,
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

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteWithdrawHook<'info>>,
    amount: u64,
) -> Result<()> {
    validate_protocols(
        &ctx.accounts.associated_protocols_info.protocols,
        ctx.accounts.protocol.key,
    )?;
    ctx.accounts
        .invoke_withdraw(ctx.remaining_accounts, amount)?;
    let total_assets = get_total_assets(
        &ctx.accounts.associated_protocols_info.protocols,
        &ctx.accounts.share_mint.key(),
        ctx.remaining_accounts,
        ctx.program_id,
    )?;

    // Compute NAV as price per share: total_assets * 10^decimals / share_supply
    let nav = if ctx.accounts.share_mint.supply == 0 {
        0
    } else {
        let precision = 10u128
            .checked_pow(ctx.accounts.share_mint.decimals as u32)
            .ok_or(HookProgramError::ArithmeticError)?;
        let ratio = u128::from(total_assets)
            .checked_mul(precision)
            .ok_or(HookProgramError::ArithmeticError)?
            .checked_div(u128::from(ctx.accounts.share_mint.supply))
            .ok_or(HookProgramError::ArithmeticError)?;
        u64::try_from(ratio).map_err(|_| HookProgramError::ArithmeticError)?
    };

    let data = nav.try_to_vec()?;
    set_return_data(&data);
    Ok(())
}

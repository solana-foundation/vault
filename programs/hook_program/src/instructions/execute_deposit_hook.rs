use anchor_lang::{
    prelude::*,
    solana_program::program::{invoke, set_return_data},
};
use anchor_spl::token_interface::Mint;

use crate::{
    errors::HookProgramError,
    state::{
        get_shares_from_assets, get_total_assets, protocol_deposit, validate_protocols,
        VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS_SEED,
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
    pub fn validate_protocols(&self) -> Result<()> {
        let protocols = &self.associated_protocols_info.protocols;

        require!(
            protocols.len() >= 2,
            HookProgramError::InsufficientAssociatedProtocols
        );

        require!(
            protocols.contains(&vault::id()),
            HookProgramError::ProtocolNotFound
        );

        require!(
            protocols.contains(&self.protocol.key()),
            HookProgramError::ProtocolNotFound
        );

        Ok(())
    }

    pub fn invoke_deposit(
        &self,
        additional_accounts: &[AccountInfo<'info>],
        deposit_amount: u64,
    ) -> Result<()> {
        let downstream_vault = additional_accounts
            .first()
            .ok_or(error!(HookProgramError::InvalidAccountData))?;

        let instruction = protocol_deposit(
            &self.protocol.key(),
            self.signer.key,
            &self.share_mint.key(),
            &downstream_vault.key(),
            &self.system_program.key(),
            deposit_amount,
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
    ctx: Context<'_, '_, '_, 'info, ExecuteDepositHook<'info>>,
    deposit_amount: u64,
) -> Result<()> {
    validate_protocols(
        &ctx.accounts.associated_protocols_info.protocols,
        ctx.accounts.protocol.key,
    )?;
    ctx.accounts
        .invoke_deposit(ctx.remaining_accounts, deposit_amount)?;
    let total_nav = get_total_assets(
        &ctx.accounts.associated_protocols_info.protocols,
        &ctx.accounts.share_mint.key(),
        ctx.remaining_accounts,
        ctx.program_id,
    )?;
    // Pre-deposit reserve balance (NAV before this deposit was added)
    let reserve_balance = total_nav
        .checked_sub(deposit_amount)
        .ok_or(HookProgramError::ArithmeticError)?;

    // Deserialize the vault (signer is the vault PDA) to read initial_price
    let vault_info = ctx.accounts.signer.to_account_info();
    let vault_data = vault_info.try_borrow_data()?;
    let mut buf: &[u8] = &vault_data;
    let vault_state = vault::state::Vault::try_deserialize(&mut buf)?;

    let shares = get_shares_from_assets(
        vault_state.initial_price,
        reserve_balance,
        ctx.accounts.share_mint.supply,
        deposit_amount,
        false, // round down for deposits
    )?;
    let mut data = shares.try_to_vec()?;
    data.extend_from_slice(&total_nav.to_le_bytes());
    set_return_data(&data);
    Ok(())
}

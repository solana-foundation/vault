use anchor_lang::prelude::*;

use crate::state::{
    NavReturnData, ProtocolDeposits, VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS,
    VAULT_NAV_DATA, VAULT_PROTOCOL_DEPOSIT,
};

#[derive(Accounts)]
pub struct UpdateNavData<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: This is the vault
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_ASSOCIATED_PROTOCOLS, vault.key().as_ref()],
        bump
    )]
    pub associated_protocols_info: Account<'info, VaultAssociatedProtocols>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + NavReturnData::INIT_SPACE,
        seeds = [VAULT_NAV_DATA, vault.key().as_ref()],
        bump
    )]
    pub nav_return_data: Account<'info, NavReturnData>,
    pub system_program: Program<'info, System>,
}

pub fn handler<'info>(ctx: Context<'_, '_, 'info, 'info, UpdateNavData<'info>>) -> Result<()> {
    let vault_key = ctx.accounts.vault.key();
    let protocols = &ctx.accounts.associated_protocols_info.protocols;
    let program_id = ctx.program_id;

    let mut total: u64 = 0;

    for protocol in protocols.iter() {
        let (pda, _bump) = Pubkey::find_program_address(
            &[
                VAULT_PROTOCOL_DEPOSIT,
                vault_key.as_ref(),
                protocol.as_ref(),
            ],
            program_id,
        );

        let amount = ctx
            .remaining_accounts
            .iter()
            .find(|a| a.key() == pda)
            .and_then(|deposit_account| Account::<ProtocolDeposits>::try_from(deposit_account).ok())
            .map(|deposit| deposit.amount)
            .unwrap_or(0);

        total = total
            .checked_add(amount)
            .ok_or_else(|| error!(ErrorCode::AccountDidNotDeserialize))?;
    }

    ctx.accounts.nav_return_data.nav = total;
    ctx.accounts.nav_return_data.update_timestamp = Clock::get()?.unix_timestamp;
    Ok(())
}

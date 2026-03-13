use anchor_lang::{prelude::*, solana_program::program::set_return_data};

use crate::state::{
    ProtocolDeposits, VaultAssociatedProtocols, VAULT_ASSOCIATED_PROTOCOLS, VAULT_PROTOCOL_DEPOSIT,
};

#[derive(Accounts)]
pub struct GetNav<'info> {
    /// CHECK: This is vault
    pub vault: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_ASSOCIATED_PROTOCOLS, vault.key().as_ref()],
        bump
    )]
    pub associated_protocols_info: Account<'info, VaultAssociatedProtocols>,
}

pub fn handler<'info>(ctx: Context<'_, '_, 'info, 'info, GetNav<'info>>) -> Result<()> {
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

        let deposit_account = ctx
            .remaining_accounts
            .iter()
            .find(|a| a.key() == pda)
            .ok_or_else(|| error!(ErrorCode::AccountNotEnoughKeys))?;

        let deposit: Account<ProtocolDeposits> = Account::try_from(deposit_account)?;

        total = total
            .checked_add(deposit.amount)
            .ok_or_else(|| error!(ErrorCode::AccountDidNotDeserialize))?;
    }
    set_return_data(&total.to_be_bytes());

    Ok(())
}

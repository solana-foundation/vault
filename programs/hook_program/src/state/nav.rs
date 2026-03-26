use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::state::VAULT_PROTOCOL_DEPOSIT_SEED;

#[account]
#[derive(InitSpace)]
pub struct VaultAssociatedProtocols {
    #[max_len(10)]
    pub protocols: Vec<Pubkey>,
    pub vault: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AssociatedProtocol {
    pub vault: Pubkey,
    pub protocol: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
}

pub fn get_nav<'info>(
    protocols: &[Pubkey],
    share_mint: &Pubkey,
    remaining_accounts: &[AccountInfo<'info>],
    program_id: &Pubkey,
) -> Result<u64> {
    let mut total: u64 = 0;

    for protocol in protocols.iter() {
        let (pda, _bump) = Pubkey::find_program_address(
            &[
                VAULT_PROTOCOL_DEPOSIT_SEED,
                share_mint.as_ref(),
                protocol.as_ref(),
            ],
            program_id,
        );

        let protocol_deposits = remaining_accounts
            .iter()
            .find(|a| a.key() == pda)
            .and_then(|deposit_account| {
                let data = deposit_account.try_borrow_data().ok()?;
                AssociatedProtocol::try_deserialize(&mut data.as_ref()).ok()
            })
            .ok_or_else(|| error!(ErrorCode::AccountDidNotDeserialize))?;

        let amount = remaining_accounts
            .iter()
            .find(|a| a.key() == protocol_deposits.token_account)
            .and_then(|token_account| {
                let data = token_account.try_borrow_data().ok()?;
                TokenAccount::try_deserialize(&mut data.as_ref()).ok()
            })
            .map(|token_account| token_account.amount)
            .unwrap_or(0);

        total = total
            .checked_add(amount)
            .ok_or_else(|| error!(ErrorCode::AccountDidNotDeserialize))?;
    }
    Ok(total)
}

use anchor_lang::prelude::*;

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
pub struct ProtocolDeposits {
    pub vault: Pubkey,
    pub protocol: Pubkey,
    pub amount: u64,
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

        let amount = remaining_accounts
            .iter()
            .find(|a| a.key() == pda)
            .and_then(|deposit_account| {
                let data = deposit_account.try_borrow_data().ok()?;
                ProtocolDeposits::try_deserialize(&mut data.as_ref()).ok()
            })
            .map(|deposit| deposit.amount)
            .unwrap_or(0);

        total = total
            .checked_add(amount)
            .ok_or_else(|| error!(ErrorCode::AccountDidNotDeserialize))?;
    }
    Ok(total)
}

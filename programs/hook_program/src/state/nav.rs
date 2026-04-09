use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::{errors::HookProgramError, state::VAULT_PROTOCOL_DEPOSIT_SEED};

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

pub fn get_total_assets<'info>(
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

/// Returns the assets/shares ratio (price per share) scaled by 10^decimals.
pub fn get_nav<'info>(
    protocols: &[Pubkey],
    share_mint: &Pubkey,
    share_supply: u64,
    share_decimals: u8,
    remaining_accounts: &[AccountInfo<'info>],
    program_id: &Pubkey,
) -> Result<u64> {
    let total_assets = get_total_assets(protocols, share_mint, remaining_accounts, program_id)?;

    if share_supply == 0 {
        return Ok(0);
    }

    let precision = 10u128.pow(share_decimals as u32);
    let ratio = u128::from(total_assets)
        .checked_mul(precision)
        .ok_or(HookProgramError::ArithmeticError)?
        .checked_div(u128::from(share_supply))
        .ok_or(HookProgramError::ArithmeticError)?;

    u64::try_from(ratio).map_err(|_| HookProgramError::ArithmeticError.into())
}

pub fn get_shares_from_assets(
    initial_price: u64,
    reserve_balance: u64,
    share_supply: u64,
    asset_amount: u64,
    round_up: bool,
) -> Result<u64> {
    let assets_times_total_supply: u128 = if share_supply == 0 {
        u128::from(initial_price)
            .checked_mul(u128::from(asset_amount))
            .ok_or(HookProgramError::ArithmeticError)?
    } else {
        u128::from(
            share_supply
                .checked_add(1)
                .ok_or(HookProgramError::ArithmeticError)?,
        )
        .checked_mul(u128::from(asset_amount))
        .ok_or(HookProgramError::ArithmeticError)?
    };
    let divisor = u128::from(
        reserve_balance
            .checked_add(1)
            .ok_or(HookProgramError::ArithmeticError)?,
    );
    let result = if round_up {
        assets_times_total_supply.div_ceil(divisor)
    } else {
        assets_times_total_supply
            .checked_div(divisor)
            .ok_or(HookProgramError::ArithmeticError)?
    };
    u64::try_from(result).or(Err(HookProgramError::ArithmeticError.into()))
}

pub fn validate_protocols<'info>(protocols: &Vec<Pubkey>, protocol: &Pubkey) -> Result<()> {
    require!(
        protocols.len() >= 2,
        HookProgramError::InsufficientAssociatedProtocols
    );

    require!(
        protocols.contains(&vault::id()),
        HookProgramError::ProtocolNotFound
    );

    require!(
        protocols.contains(protocol),
        HookProgramError::ProtocolNotFound
    );

    Ok(())
}

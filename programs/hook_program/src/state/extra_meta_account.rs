use crate::state::{
    anchor_discriminator, DEPOSIT_ACCOUNT_METAS_SEED, EXTRA_ACCOUNT_METAS_SEED,
    WITHDRAW_ACCOUNT_METAS_SEED,
};
use anchor_lang::prelude::{AccountMeta, ProgramError, Pubkey};
use spl_discriminator::{ArrayDiscriminator, SplDiscriminate};
use spl_tlv_account_resolution::solana_instruction::Instruction;

pub enum VaultStandardInstruction {
    DepositHook,
    WithdrawHook,
}

#[derive(SplDiscriminate)]
#[discriminator_hash_input("vault-standard:deposit-hook")]
pub struct DepositHookInstruction;

#[derive(SplDiscriminate)]
#[discriminator_hash_input("vault-standard:withdraw-hook")]
pub struct WithdrawHookInstruction;

impl VaultStandardInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, _) = data.split_at(ArrayDiscriminator::LENGTH);
        match discriminator {
            DepositHookInstruction::SPL_DISCRIMINATOR_SLICE => Ok(Self::DepositHook),
            WithdrawHookInstruction::SPL_DISCRIMINATOR_SLICE => Ok(Self::WithdrawHook),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        match self {
            Self::DepositHook => anchor_discriminator("global", "deposit").to_vec(),
            Self::WithdrawHook => anchor_discriminator("global", "withdraw").to_vec(),
        }
    }
}

pub fn get_deposit_hook_extra_account_metas_address(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_deposit_hook_extra_account_metas_address_and_bump_seed(mint, program_id).0
}

pub fn get_deposit_hook_extra_account_metas_address_and_bump_seed(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&collect_deposit_hook_extra_account_metas(mint), program_id)
}

pub fn collect_deposit_hook_extra_account_metas(mint: &Pubkey) -> [&[u8]; 3] {
    [
        EXTRA_ACCOUNT_METAS_SEED,
        DEPOSIT_ACCOUNT_METAS_SEED,
        mint.as_ref(),
    ]
}

pub fn get_withdraw_hook_extra_account_metas_address(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_withdraw_hook_extra_account_metas_address_and_bump_seed(mint, program_id).0
}

pub fn get_withdraw_hook_extra_account_metas_address_and_bump_seed(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&collect_withdraw_hook_extra_account_metas(mint), program_id)
}

pub fn collect_withdraw_hook_extra_account_metas(mint: &Pubkey) -> [&[u8]; 3] {
    [
        EXTRA_ACCOUNT_METAS_SEED,
        WITHDRAW_ACCOUNT_METAS_SEED,
        mint.as_ref(),
    ]
}

pub fn protocol_deposit(
    program_id: &Pubkey,
    signer: &Pubkey,
    share_mint: &Pubkey,
    vault: &Pubkey,
    system_program: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = VaultStandardInstruction::DepositHook.pack();
    data.extend_from_slice(&amount.to_le_bytes());
    let accounts = vec![
        AccountMeta::new(*signer, true),
        AccountMeta::new_readonly(*share_mint, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*system_program, false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

pub fn protocol_withdraw(
    program_id: &Pubkey,
    signer: &Pubkey,
    share_mint: &Pubkey,
    vault: &Pubkey,
    system_program: &Pubkey,
) -> Instruction {
    let mut data = VaultStandardInstruction::WithdrawHook.pack();
    data.extend_from_slice(&0u64.to_le_bytes());
    let accounts = vec![
        AccountMeta::new(*signer, true),
        AccountMeta::new_readonly(*share_mint, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*system_program, false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

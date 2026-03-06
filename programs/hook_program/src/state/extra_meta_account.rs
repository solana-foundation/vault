use anchor_lang::{
    prelude::{AccountMeta, ProgramError, Pubkey},
    solana_program::instruction::Instruction,
};

use spl_discriminator::{ArrayDiscriminator, SplDiscriminate};
pub enum VaultStandardInstruction {
    DepositHook,
}

#[derive(SplDiscriminate)]
#[discriminator_hash_input("vault-standard:deposit-hook")]
pub struct DepositHookInstruction;

impl VaultStandardInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, _) = data.split_at(ArrayDiscriminator::LENGTH);
        match discriminator {
            DepositHookInstruction::SPL_DISCRIMINATOR_SLICE => Ok(Self::DepositHook),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        match self {
            Self::DepositHook => DepositHookInstruction::SPL_DISCRIMINATOR_SLICE.to_vec(),
        }
    }
}

mod generated;

pub use generated::{instructions::*, programs::*};

pub use solana_pubkey::Pubkey;

#[cfg(feature = "solana-sdk")]
pub mod sdk {
    use super::*;

    pub fn program_id() -> solana_sdk::pubkey::Pubkey {
        solana_sdk::pubkey::Pubkey::new_from_array(DUMMY_PROTOCOL_ID.to_bytes())
    }

    pub trait IntoSdkInstruction {
        fn into_sdk_instruction(self) -> solana_sdk::instruction::Instruction;
    }

    impl IntoSdkInstruction for solana_instruction::Instruction {
        fn into_sdk_instruction(self) -> solana_sdk::instruction::Instruction {
            solana_sdk::instruction::Instruction {
                program_id: solana_sdk::pubkey::Pubkey::new_from_array(self.program_id.to_bytes()),
                accounts: self
                    .accounts
                    .into_iter()
                    .map(|meta| solana_sdk::instruction::AccountMeta {
                        pubkey: solana_sdk::pubkey::Pubkey::new_from_array(meta.pubkey.to_bytes()),
                        is_signer: meta.is_signer,
                        is_writable: meta.is_writable,
                    })
                    .collect(),
                data: self.data,
            }
        }
    }
}

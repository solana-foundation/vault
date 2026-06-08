mod generated;

pub mod extensions;

pub use generated::{
    accounts::*, errors::*, instructions::*, programs::*, shared, shared::*, types::*,
};

pub use solana_pubkey::Pubkey;

#[cfg(feature = "litesvm")]
pub mod lite {
    use super::*;
    use litesvm;
    use solana_sdk::{signers::Signers, transaction::Transaction};

    pub trait SendTransaction {
        fn send_transaction<T: Signers + ?Sized>(
            self,
            svm: &mut litesvm::LiteSVM,
            payer: &Pubkey,
            signers: &T,
        ) -> litesvm::types::TransactionResult;
    }

    impl SendTransaction for solana_instruction::Instruction {
        fn send_transaction<T: Signers + ?Sized>(
            self,
            svm: &mut litesvm::LiteSVM,
            payer: &Pubkey,
            signers: &T,
        ) -> litesvm::types::TransactionResult {
            let tx = Transaction::new_signed_with_payer(
                &[self],
                Some(payer),
                signers,
                svm.latest_blockhash(),
            );
            svm.send_transaction(tx)
        }
    }
}

#[cfg(feature = "solana-sdk")]
pub mod sdk {
    use super::*;

    pub fn program_id() -> solana_sdk::pubkey::Pubkey {
        solana_sdk::pubkey::Pubkey::new_from_array(ASYNC_VAULT_ID.to_bytes())
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

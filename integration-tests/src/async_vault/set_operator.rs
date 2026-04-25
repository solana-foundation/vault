use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};

use crate::helper_functions::{assert_error_code, set_operator, setup_async_vault};

#[test]
fn test_set_operator_succeeds() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let operator = Keypair::new();
    svm.airdrop(&operator.pubkey(), 1_000_000_000).unwrap();

    set_operator(
        &mut svm,
        &authority,
        &operator,
        share_mint.pubkey(),
        vault_pubkey,
    )
    .expect("set operator should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_data = vault_account.data();
    let vault_config = Vault::from_bytes(vault_data).unwrap();
    assert_eq!(vault_config.operator, Some(operator.pubkey()));
}

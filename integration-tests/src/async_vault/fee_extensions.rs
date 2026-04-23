use async_vault_client::{sdk::program_id, FeeType, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};

use crate::helper_functions::{
    assert_error_code, init_deposit_fee, init_withdrawal_fee, setup_async_vault,
    update_deposit_fee, update_withdrawal_fee,
};

#[test]
fn test_initialize_and_update_deposit_fee() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee.clone(),
    )
    .expect("init deposit fee should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_data = vault_account.data();
    let vault_config = Vault::from_bytes(vault_data).unwrap();
    assert!(!vault_config.initialized);
    assert!(vault_data.len() > Vault::LEN);

    let new_deposit_fee = FeeType::Percentage { bps: 500 };
    update_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        new_deposit_fee.clone(),
    )
    .expect("update deposit fee should succeed");
}

#[test]
fn test_initialize_and_update_withdrawal_fee() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let withdrawal_fee = FeeType::Percentage { bps: 200 };
    init_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee.clone(),
    )
    .expect("init withdrawal fee should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_data = vault_account.data();
    assert!(vault_data.len() > Vault::LEN);

    let new_withdrawal_fee = FeeType::FixedAmount { amount: 50 };
    update_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        new_withdrawal_fee.clone(),
    )
    .expect("update withdrawal fee should succeed");
}

#[test]
fn test_initialize_both_fees() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee.clone(),
    )
    .expect("init deposit fee should succeed");

    let withdrawal_fee = FeeType::Percentage { bps: 300 };
    init_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee.clone(),
    )
    .expect("init withdrawal fee should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_data = vault_account.data();
    assert!(vault_data.len() > Vault::LEN + 13);
}

#[test]
fn test_initialize_deposit_fee_duplicate_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee.clone(),
    )
    .expect("first init should succeed");

    svm.expire_blockhash();

    let result = init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee,
    );
    assert_error_code(&result.unwrap_err(), 6005, "ExtensionAlreadyInitialized");
}

#[test]
fn test_initialize_withdrawal_fee_duplicate_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let withdrawal_fee = FeeType::Percentage { bps: 100 };
    init_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee.clone(),
    )
    .expect("first init should succeed");

    svm.expire_blockhash();

    let result = init_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee,
    );
    assert_error_code(&result.unwrap_err(), 6005, "ExtensionAlreadyInitialized");
}

#[test]
fn test_update_deposit_fee_before_init_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    let result = update_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee,
    );
    assert_error_code(&result.unwrap_err(), 6006, "UninitializedExtension");
}

#[test]
fn test_update_withdrawal_fee_before_init_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let withdrawal_fee = FeeType::Percentage { bps: 100 };
    let result = update_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee,
    );
    assert_error_code(&result.unwrap_err(), 6006, "UninitializedExtension");
}

#[test]
fn test_initialize_deposit_fee_invalid_bps_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::Percentage { bps: 10_001 };
    let result = init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee,
    );
    assert_error_code(&result.unwrap_err(), 6000, "FeeBPSLimitReached");
}

#[test]
fn test_initialize_withdrawal_fee_invalid_bps_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let withdrawal_fee = FeeType::Percentage { bps: 10_001 };
    let result = init_withdrawal_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        withdrawal_fee,
    );
    assert_error_code(&result.unwrap_err(), 6000, "FeeBPSLimitReached");
}

#[test]
fn test_initialize_fee_unauthorized_signer_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    let result = init_deposit_fee(
        &mut svm,
        &unauthorized,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee,
    );
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

#[test]
fn test_update_fee_unauthorized_signer_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    init_deposit_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fee,
    )
    .expect("init should succeed");

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let new_fee = FeeType::FixedAmount { amount: 200 };
    let result = update_deposit_fee(
        &mut svm,
        &unauthorized,
        share_mint.pubkey(),
        vault_pubkey,
        new_fee,
    );
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

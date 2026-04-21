use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::async_vault::{
    constants::{PENDING_VAULT_SEED, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{
        async_vault_program_id, create_async_vault, create_mint, initialize_async_vault,
        AsyncVaultAccount,
    },
};

fn setup_vault(
    svm: &mut LiteSVM,
) -> (Keypair, Keypair, Keypair, Keypair, Keypair, Pubkey, Pubkey) {
    let initial_price = 100_000_000;

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(svm, &mint_authority, &asset_mint);
    create_mint(svm, &mint_authority, &share_mint);

    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[RESERVE_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );
    let (pending_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_VAULT_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );

    create_async_vault(
        svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
        initial_price,
        true,
        true,
        token::ID,
        token::ID,
    )
    .expect("async vault creation should succeed");

    (
        authority,
        payer,
        mint_authority,
        asset_mint,
        share_mint,
        vault_pubkey,
        pending_vault_pubkey,
    )
}

fn create_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(async_vault_program_id(), program_bytes)
        .unwrap();
    svm
}

#[test]
fn test_initialize_vault() {
    let mut svm = create_svm();
    let (authority, _payer, _mint_authority, _asset_mint, share_mint, vault_pubkey, _) =
        setup_vault(&mut svm);

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_before = AsyncVaultAccount::from_account_data(vault_account.data());
    assert!(!vault_before.initialized, "Vault should start uninitialized");

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize_vault should succeed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_after = AsyncVaultAccount::from_account_data(vault_account.data());
    assert!(
        vault_after.initialized,
        "Vault should be initialized after calling initialize_vault"
    );
}

#[test]
fn test_initialize_vault_wrong_authority() {
    let mut svm = create_svm();
    let (_authority, _payer, _mint_authority, _asset_mint, share_mint, vault_pubkey, _) =
        setup_vault(&mut svm);

    let wrong_authority = Keypair::new();
    svm.airdrop(&wrong_authority.pubkey(), 1_000_000_000)
        .unwrap();

    let result =
        initialize_async_vault(&mut svm, &wrong_authority, share_mint.pubkey(), vault_pubkey);
    assert!(
        result.is_err(),
        "initialize_vault should fail with wrong authority"
    );
}

#[test]
fn test_initialize_vault_already_initialized() {
    let mut svm = create_svm();
    let (authority, _payer, _mint_authority, _asset_mint, share_mint, vault_pubkey, _) =
        setup_vault(&mut svm);

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("first initialize_vault should succeed");

    let result = initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey);
    assert!(
        result.is_err(),
        "initialize_vault should fail when already initialized"
    );
}

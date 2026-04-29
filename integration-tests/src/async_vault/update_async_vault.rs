use anchor_spl::token;
use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, set_up_async_vault, update_async_vault};

#[test_case(true; "pause vault")]
#[test_case(false ; "unpause vault")]
fn test_update_async_vault(paused: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0, 100_000_000);

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_before = Vault::from_bytes(vault_account.data()).unwrap();

    update_async_vault(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        paused,
    )
    .expect("update vault should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_after = Vault::from_bytes(vault_account.data()).unwrap();

    assert_eq!(vault_after.paused, paused);

    assert_eq!(vault_after.authority, vault_before.authority);
    assert_eq!(vault_after.asset_mint, vault_before.asset_mint);
    assert_eq!(vault_after.share_mint, vault_before.share_mint);
    assert_eq!(vault_after.initial_price, vault_before.initial_price);
    assert_eq!(vault_after.nav, vault_before.nav);
    assert_eq!(vault_after.nav_version, vault_before.nav_version);
}

#[test]
fn test_update_async_vault_unauthorized_signer_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (
        _authority,
        _payer,
        _mint_authority,
        _asset_mint,
        share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0, 100_000_000);

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let result = update_async_vault(
        &mut svm,
        &unauthorized,
        share_mint.pubkey(),
        vault_pubkey,
        true,
    );

    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{
    accept_authority_invitation, assert_error_code, invite_new_authority, setup_async_vault,
};

#[test_case(true ; "succeeds")]
#[test_case(false ; "unauthorized signer fails")]
fn test_invite_new_authority(use_valid_authority: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let new_authority = Keypair::new();

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let effective_authority = if use_valid_authority {
        &authority
    } else {
        &unauthorized
    };

    let result = invite_new_authority(
        &mut svm,
        effective_authority,
        new_authority.pubkey(),
        vault_pubkey,
    );

    if use_valid_authority {
        result.expect("invite new authority should succeed");

        let vault_account = svm.get_account(&vault_pubkey).unwrap();
        let vault_data = Vault::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_data.pending_authority, Some(new_authority.pubkey()));
    } else {
        assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
    }
}

#[test]
fn test_invite_new_authority_overwrites_pending() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, _, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let first_candidate = Keypair::new();
    invite_new_authority(&mut svm, &authority, first_candidate.pubkey(), vault_pubkey)
        .expect("first invite should succeed");

    svm.expire_blockhash();

    let second_candidate = Keypair::new();
    invite_new_authority(
        &mut svm,
        &authority,
        second_candidate.pubkey(),
        vault_pubkey,
    )
    .expect("second invite should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_data = Vault::from_bytes(vault_account.data()).unwrap();
    assert_eq!(
        vault_data.pending_authority,
        Some(second_candidate.pubkey())
    );
}

#[test_case(true, true ; "succeeds")]
#[test_case(false, true ; "no pending authority fails")]
#[test_case(true, false ; "wrong new authority fails")]
fn test_accept_authority_invitation(invite_first: bool, use_correct_new_authority: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let new_authority = Keypair::new();
    svm.airdrop(&new_authority.pubkey(), 1_000_000_000).unwrap();

    if invite_first {
        invite_new_authority(&mut svm, &authority, new_authority.pubkey(), vault_pubkey)
            .expect("invite should succeed");
        svm.expire_blockhash();
    }

    let wrong_new_authority = Keypair::new();
    svm.airdrop(&wrong_new_authority.pubkey(), 1_000_000_000)
        .unwrap();

    let effective_new_authority = if use_correct_new_authority {
        &new_authority
    } else {
        &wrong_new_authority
    };

    let result = accept_authority_invitation(&mut svm, effective_new_authority, vault_pubkey);

    let should_succeed = invite_first && use_correct_new_authority;

    if should_succeed {
        result.expect("accept authority invitation should succeed");

        let vault_account = svm.get_account(&vault_pubkey).unwrap();
        let vault_data = Vault::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_data.authority, new_authority.pubkey());
        assert_eq!(vault_data.pending_authority, None);
    } else {
        let err = result.unwrap_err();
        if !invite_first {
            assert_error_code(&err, 6012, "NoPendingAuthority");
        } else {
            assert_error_code(&err, 6001, "UnauthorizedSigner");
        }
    }
}

#[test]
fn test_full_authority_transfer_old_authority_loses_access() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let new_authority = Keypair::new();
    svm.airdrop(&new_authority.pubkey(), 1_000_000_000).unwrap();

    invite_new_authority(&mut svm, &authority, new_authority.pubkey(), vault_pubkey)
        .expect("invite should succeed");

    svm.expire_blockhash();

    accept_authority_invitation(&mut svm, &new_authority, vault_pubkey)
        .expect("accept should succeed");

    svm.expire_blockhash();

    let another = Keypair::new();
    let result = invite_new_authority(&mut svm, &authority, another.pubkey(), vault_pubkey);
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");

    svm.expire_blockhash();

    invite_new_authority(&mut svm, &new_authority, another.pubkey(), vault_pubkey)
        .expect("new authority should be able to invite");
}

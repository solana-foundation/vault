use anchor_spl::token;
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, AcceptAuthorityInvitationBuilder,
    InviteNewAuthorityBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::async_helper_functions::{assert_error_code, set_up_async_vault};

#[test_case(true ; "succeeds")]
#[test_case(false ; "unauthorized signer fails")]
fn test_invite_new_authority(use_valid_authority: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let new_authority = Keypair::new();

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let effective_authority = if use_valid_authority {
        &authority
    } else {
        &unauthorized
    };

    let result = InviteNewAuthorityBuilder::new()
        .authority(effective_authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(new_authority.pubkey())
        .instruction()
        .send_transaction(
            &mut svm,
            &effective_authority.pubkey(),
            &[effective_authority],
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
    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let first_candidate = Keypair::new();
    InviteNewAuthorityBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(first_candidate.pubkey())
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("first invite should succeed");

    svm.expire_blockhash();

    let second_candidate = Keypair::new();
    InviteNewAuthorityBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(second_candidate.pubkey())
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
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
    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let new_authority = Keypair::new();
    svm.airdrop(&new_authority.pubkey(), 1_000_000_000).unwrap();

    if invite_first {
        InviteNewAuthorityBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .new_authority(new_authority.pubkey())
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
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

    let result = AcceptAuthorityInvitationBuilder::new()
        .new_authority(effective_new_authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(
            &mut svm,
            &effective_new_authority.pubkey(),
            &[effective_new_authority],
        );

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
    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let new_authority = Keypair::new();
    svm.airdrop(&new_authority.pubkey(), 1_000_000_000).unwrap();

    InviteNewAuthorityBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(new_authority.pubkey())
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("invite should succeed");

    svm.expire_blockhash();

    AcceptAuthorityInvitationBuilder::new()
        .new_authority(new_authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &new_authority.pubkey(), &[&new_authority])
        .expect("accept should succeed");

    svm.expire_blockhash();

    let another = Keypair::new();
    let result = InviteNewAuthorityBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(another.pubkey())
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority]);
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");

    svm.expire_blockhash();

    InviteNewAuthorityBuilder::new()
        .authority(new_authority.pubkey())
        .vault(vault_pubkey)
        .new_authority(another.pubkey())
        .instruction()
        .send_transaction(&mut svm, &new_authority.pubkey(), &[&new_authority])
        .expect("new authority should be able to invite");
}

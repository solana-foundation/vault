use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction},
    ApproveRequestBuilder, Request, RequestState, Vault,
};
use borsh::BorshSerialize;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, signature::Keypair, signer::Signer, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    approve_request, assert_error_code, create_deposit_request_ix, initialize_async_vault,
    set_up_async_vault, update_async_vault, update_vault_nav,
};

#[test]
fn test_approve_request_success() {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let user_amount = 1_000_000_000;
    let (
        authority,
        _payer,
        _mint_authority,
        asset_mint,
        share_mint,
        user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(
        &mut svm,
        token::ID,
        None,
        token::ID,
        user_amount,
        100_000_000,
    );
    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    let deposit_amount = 1_000_000;
    let request_keypair = Keypair::new();

    let ix = create_deposit_request_ix(
        &user,
        &request_keypair,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        deposit_amount,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("create deposit request should succeed");

    // Update NAV to a new value — approve_request should lock in this newer NAV
    let new_nav = 200u128;
    update_vault_nav(&mut svm, &authority, vault_pubkey, new_nav)
        .expect("update_vault_nav should succeed");

    // Approve the request
    approve_request(&mut svm, &authority, vault_pubkey, request_keypair.pubkey())
        .expect("approve_request should succeed");

    // Assert all state changes
    let vault_after = Vault::from_bytes(
        svm.get_account(&vault_pubkey)
            .expect("vault should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(vault_after.pending_async_requests, 0);

    let request_after = Request::from_bytes(
        svm.get_account(&request_keypair.pubkey())
            .expect("request should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(request_after.request_state, RequestState::Claimable);
    assert_eq!(request_after.price, new_nav);
}

#[test_case(false, true, false, 6001 ; "unauthorized signer")]
#[test_case(true, false, false, 6003 ; "paused vault")]
#[test_case(false, false, true, 6021 ; "request not in pending state")]
#[test_case(false, false, false, 6029 ; "nav not set")]
fn test_approve_request_fails(
    pause_vault: bool,
    use_wrong_signer: bool,
    override_to_claimable: bool,
    expected_error_code: u32,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let user_amount = 1_000_000_000;
    let (
        authority,
        _payer,
        _mint_authority,
        asset_mint,
        share_mint,
        user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(
        &mut svm,
        token::ID,
        None,
        token::ID,
        user_amount,
        100_000_000,
    );
    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    let request_keypair = Keypair::new();
    let ix = create_deposit_request_ix(
        &user,
        &request_keypair,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        1_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("create deposit request should succeed");

    if override_to_claimable {
        let mut account = svm.get_account(&request_keypair.pubkey()).unwrap();
        let mut request = Request::from_bytes(account.data()).unwrap();
        request.request_state = RequestState::Claimable;
        let mut buf = Vec::new();
        request.serialize(&mut buf).unwrap();
        account.data = buf;
        svm.set_account(request_keypair.pubkey(), account).unwrap();
    }

    if pause_vault {
        update_async_vault(
            &mut svm,
            &authority,
            share_mint.pubkey(),
            vault_pubkey,
            true,
        )
        .expect("pause vault should succeed");
    }

    let (signer, authority_key) = if use_wrong_signer {
        (&user, user.pubkey())
    } else {
        (&authority, authority.pubkey())
    };

    let ix = ApproveRequestBuilder::new()
        .authority(authority_key)
        .vault(vault_pubkey)
        .request(request_keypair.pubkey())
        .instruction()
        .into_sdk_instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer],
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert_error_code(&err, expected_error_code, "");
}

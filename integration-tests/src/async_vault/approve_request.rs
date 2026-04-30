use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{sdk::program_id, ApproveRequestBuilder, Request, RequestState, Vault};
use borsh::BorshSerialize;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, signature::Keypair, signer::Signer, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    approve_request, assert_error_code, create_deposit_request, get_token_account_amount,
    initialize_async_vault, set_up_async_vault, update_async_vault, update_vault_nav,
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
        reserve_pubkey,
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

    create_deposit_request(
        &mut svm,
        &user,
        &request_keypair,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        deposit_amount,
    )
    .expect("create deposit request should succeed");

    // Update NAV — approve_request should lock in this NAV and use it for conversion
    let new_nav = 200u128;
    update_vault_nav(&mut svm, &authority, vault_pubkey, new_nav)
        .expect("update_vault_nav should succeed");

    // Snapshot balances before approve
    let pending_before = get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap());
    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());

    // Approve the request
    approve_request(
        &mut svm,
        &authority,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        token::ID,
    )
    .expect("approve_request should succeed");

    // Assert vault state changes
    let vault_after = Vault::from_bytes(
        svm.get_account(&vault_pubkey)
            .expect("vault should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(vault_after.pending_async_requests, 0);
    assert_eq!(
        vault_after.total_asset_balance, deposit_amount,
        "total_asset_balance should be incremented by deposit amount"
    );

    let request_after = Request::from_bytes(
        svm.get_account(&request_keypair.pubkey())
            .expect("request should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(request_after.request_state, RequestState::Claimable);
    assert_eq!(request_after.price, new_nav);
    // shares = deposit_amount * 10^9 / nav = 1_000_000 * 1_000_000_000 / 200 = 5_000_000_000_000
    let expected_shares = deposit_amount as u128 * 1_000_000_000 / new_nav;
    assert_eq!(
        request_after.amount, expected_shares as u64,
        "request.amount should be updated to claimable shares"
    );

    // Assets should have moved from pending to reserve
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        pending_before - deposit_amount,
        "pending_vault should be drained"
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap()),
        reserve_before + deposit_amount,
        "reserve should receive the deposited assets"
    );
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
        reserve_pubkey,
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
    create_deposit_request(
        &mut svm,
        &user,
        &request_keypair,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        1_000_000,
    )
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
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault_token_account(reserve_pubkey)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token::ID)
        .instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer],
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert_error_code(&err, expected_error_code, "");
}

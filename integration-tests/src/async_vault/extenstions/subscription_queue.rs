use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    extensions::subscription_queue, lite::SendTransaction, sdk::program_id, ApproveRequestBuilder,
    CancelQueuedDepositRequestBuilder, CreateDepositRequestBuilder,
    InitializeSubscriptionQueueBuilder, InitializeVaultBuilder as InitializeAsyncVaultBuilder,
    RejectRequestBuilder, Request, RequestArgs, RequestState, SkipCanceledQueueRequestBuilder,
    UpdateVaultNavBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::async_helper_functions::{approve_request_args, assert_error_code, set_up_async_vault};

const NAV: u128 = 1_000_000_000;

// ── Test setup ────────────────────────────────────────────────────────────────

#[allow(clippy::type_complexity)]
fn setup(
    with_subscription_queue: bool,
) -> (
    LiteSVM,
    Keypair, // authority
    Keypair, // asset_mint
    Keypair, // share_mint
    Keypair, // user
    Pubkey,  // vault_pubkey
    Pubkey,  // reserve_pubkey
    Pubkey,  // pending_vault_pubkey
    Pubkey,  // user_token_account
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

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
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 1_000_000_000);

    if with_subscription_queue {
        InitializeSubscriptionQueueBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize_subscription_queue should succeed");
    }

    InitializeAsyncVaultBuilder::new()
        .share_mint(share_mint.pubkey())
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");

    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(NAV)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    (
        svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    )
}

fn create_deposit_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    user_token_account: Pubkey,
    pending_vault_pubkey: Pubkey,
    amount: u64,
) -> Keypair {
    let request_keypair = Keypair::new();
    CreateDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(user_token_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID)
        .args(RequestArgs {
            amount,
            operator: None,
        })
        .instruction()
        .send_transaction(svm, &user.pubkey(), &[user, &request_keypair])
        .expect("create_deposit_request should succeed");
    request_keypair
}

fn approve_deposit_request(
    svm: &mut LiteSVM,
    authority: &Keypair,
    vault_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    pending_vault_pubkey: Pubkey,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    request_pubkey: Pubkey,
) -> litesvm::types::TransactionResult {
    let (owner, request_type, amount, created_at, nav_update_version) =
        approve_request_args(svm, &request_pubkey);
    ApproveRequestBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .request(request_pubkey)
        .owner(owner)
        .request_type(request_type)
        .amount(amount)
        .created_at(created_at)
        .nav_update_version(nav_update_version)
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .vault_token_account(reserve_pubkey)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token::ID)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

fn reject_deposit_request(
    svm: &mut LiteSVM,
    authority: &Keypair,
    user_pubkey: Pubkey,
    vault_pubkey: Pubkey,
    pending_vault_pubkey: Pubkey,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    request_pubkey: Pubkey,
    user_token_account: Pubkey,
) -> litesvm::types::TransactionResult {
    RejectRequestBuilder::new()
        .authority(authority.pubkey())
        .user(user_pubkey)
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .request(request_pubkey)
        .vault(vault_pubkey)
        .user_token_account(Some(user_token_account))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID))
        .user_share_account(None)
        .share_token_program(None)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_subscription_queue_state_is_zeroed() {
    let (svm, _authority, _asset_mint, _share_mint, _user, vault_pubkey, ..) = setup(true);

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).expect("vault exists").data())
            .expect("SubscriptionQueue extension should be present");

    assert_eq!(
        queue.all_time_total_subscription_requests, 0,
        "all_time_total should start at 0"
    );
    assert_eq!(
        queue.last_processed_subscription_request_index, 0,
        "last_processed should start at 0"
    );
}

#[test_case(true, false, 6004, "VaultAlreadyInitialized" ; "after_vault_init")]
#[test_case(false, true, 6005, "ExtensionAlreadyInitialized" ; "duplicate")]
fn test_initialize_subscription_queue_fails(
    init_vault_first: bool,
    init_extension_first: bool,
    expected_error: u32,
    expected_name: &str,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
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
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 1_000_000_000);

    if init_vault_first {
        InitializeAsyncVaultBuilder::new()
            .share_mint(share_mint.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize vault should succeed");
    }

    if init_extension_first {
        InitializeSubscriptionQueueBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("first initialize should succeed");
        svm.expire_blockhash();
    }

    let err = InitializeSubscriptionQueueBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

#[test]
fn test_create_deposit_request_increments_counter_and_sets_request_id() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        _reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    let request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_subscription_requests, 1,
        "counter should be 1 after first request"
    );
    assert_eq!(
        queue.last_processed_subscription_request_index, 0,
        "last_processed unchanged"
    );

    let id_1 =
        subscription_queue::get_request_state(svm.get_account(&request_1.pubkey()).unwrap().data())
            .expect("request 1 should have SubscriptionQueueRequest extension")
            .id;
    assert_eq!(id_1, 1, "first request should have id=1");

    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_subscription_requests, 2,
        "counter should be 2 after second request"
    );
    assert_eq!(
        queue.last_processed_subscription_request_index, 0,
        "last_processed still unchanged"
    );

    let id_2 =
        subscription_queue::get_request_state(svm.get_account(&request_2.pubkey()).unwrap().data())
            .expect("request 2 should have SubscriptionQueueRequest extension")
            .id;
    assert_eq!(id_2, 2, "second request should have id=2");

    // Cleanup: ensure vault state is consistent
    let vault = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(vault.pending_async_requests, 2);

    // Approve request 1 to validate end-to-end state
    approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        _reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_1.pubkey(),
    )
    .expect("approve request 1 should succeed");

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_subscription_requests, 2,
        "all_time_total unchanged after approve"
    );
    assert_eq!(
        queue.last_processed_subscription_request_index, 1,
        "last_processed should be 1 after approving request 1"
    );

    let request_1_state =
        Request::from_bytes(svm.get_account(&request_1.pubkey()).unwrap().data()).unwrap();
    assert_eq!(request_1_state.request_state, RequestState::Claimable);
}

#[test]
fn test_approve_request_out_of_order_fails() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    // Create request 1 and request 2
    let _request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    // Attempt to approve request 2 before request 1 — must fail
    let err = approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_2.pubkey(),
    )
    .unwrap_err();
    assert_error_code(&err, 6029, "SubscriptionQueueOutOfOrder");

    // Vault state must be unchanged
    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_subscription_request_index, 0,
        "last_processed must not change on failed approve"
    );
}

#[test]
fn test_reject_request_out_of_order_fails() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        _reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    let _request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    let err = reject_deposit_request(
        &mut svm,
        &authority,
        user.pubkey(),
        vault_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_2.pubkey(),
        user_token_account,
    )
    .unwrap_err();
    assert_error_code(&err, 6029, "SubscriptionQueueOutOfOrder");

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_subscription_request_index, 0,
        "last_processed must not change on failed reject"
    );
}

#[test]
fn test_fifo_ordering_approve_then_reject() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    let request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_3 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    // Approve request 1
    approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_1.pubkey(),
    )
    .expect("approve request 1 should succeed");
    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        1
    );

    // Reject request 2
    reject_deposit_request(
        &mut svm,
        &authority,
        user.pubkey(),
        vault_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_2.pubkey(),
        user_token_account,
    )
    .expect("reject request 2 should succeed");
    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        2
    );
    assert!(
        svm.get_account(&request_2.pubkey()).is_none(),
        "rejected request account should be closed"
    );

    // Approve request 3
    approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_3.pubkey(),
    )
    .expect("approve request 3 should succeed");
    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        3
    );
    assert_eq!(
        Request::from_bytes(svm.get_account(&request_3.pubkey()).unwrap().data())
            .unwrap()
            .request_state,
        RequestState::Claimable
    );
}

// ── cancel_queued_deposit_request helpers ─────────────────────────────────────

fn cancel_queued_deposit_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    user_token_account: Pubkey,
    pending_vault_pubkey: Pubkey,
    request_pubkey: Pubkey,
) -> litesvm::types::TransactionResult {
    CancelQueuedDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .vault(vault_pubkey)
        .request(request_pubkey)
        .user_token_account(user_token_account)
        .asset_pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID)
        .instruction()
        .send_transaction(svm, &user.pubkey(), &[user])
}

fn skip_canceled_subscription_request(
    svm: &mut LiteSVM,
    caller: &Keypair,
    vault_pubkey: Pubkey,
    request_pubkey: Pubkey,
    owner: Pubkey,
) -> litesvm::types::TransactionResult {
    SkipCanceledQueueRequestBuilder::new()
        .vault(vault_pubkey)
        .request(request_pubkey)
        .owner(owner)
        .instruction()
        .send_transaction(svm, &caller.pubkey(), &[caller])
}

// ── cancel_queued_deposit_request tests ──────────────────────────────────────

/// Canceling the only queued request and then creating a new one should not stall the queue.
#[test]
fn test_cancel_queued_deposit_then_new_request_can_be_approved() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    let initial_balance = crate::async_helper_functions::get_token_account_amount(
        &svm.get_account(&user_token_account).unwrap(),
    );

    let request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    // Cancel request #1 via the queued cancel path
    cancel_queued_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        request_1.pubkey(),
    )
    .expect("cancel_queued_deposit_request should succeed");

    // Request account must still be open (tombstone)
    let req_data = svm
        .get_account(&request_1.pubkey())
        .expect("tombstone request account must remain open");
    assert_eq!(
        Request::from_bytes(req_data.data()).unwrap().request_state,
        RequestState::Canceled,
        "request state must be Canceled"
    );

    // Assets must be refunded
    assert_eq!(
        crate::async_helper_functions::get_token_account_amount(
            &svm.get_account(&user_token_account).unwrap()
        ),
        initial_balance,
        "assets must be fully refunded after cancel"
    );
    assert_eq!(
        crate::async_helper_functions::get_token_account_amount(
            &svm.get_account(&pending_vault_pubkey).unwrap()
        ),
        0,
        "pending vault must be empty after cancel"
    );

    // Queue counter incremented; last_processed still 0
    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(queue.all_time_total_subscription_requests, 1);
    assert_eq!(queue.last_processed_subscription_request_index, 0);

    // Advance queue past the tombstone — anyone can call this
    let authority_clone = authority;
    skip_canceled_subscription_request(
        &mut svm,
        &authority_clone,
        vault_pubkey,
        request_1.pubkey(),
        user.pubkey(),
    )
    .expect("skip_canceled_queue_request should succeed");

    // Tombstone account must be closed
    assert!(
        svm.get_account(&request_1.pubkey()).is_none(),
        "tombstone account must be closed after skip"
    );

    let queue =
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_subscription_request_index, 1,
        "queue must have advanced past canceled request"
    );

    // Create request #2 (ID=2) and approve it — must succeed
    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    approve_deposit_request(
        &mut svm,
        &authority_clone,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_2.pubkey(),
    )
    .expect("approve request #2 should succeed after queue unblock");

    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        2,
        "last_processed must reach 2 after approving request #2"
    );
}

/// Cancel a request in the middle (#2 of #1, #2, #3). Process #1 normally, then skip the
/// tombstone for #2, then process #3 — all must succeed.
#[test]
fn test_cancel_middle_request_queue_unblocks_after_skip() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(true);

    let request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_2 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );
    let request_3 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    // Cancel the middle request
    cancel_queued_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        request_2.pubkey(),
    )
    .expect("cancel_queued_deposit_request for #2 should succeed");

    assert_eq!(
        Request::from_bytes(svm.get_account(&request_2.pubkey()).unwrap().data())
            .unwrap()
            .request_state,
        RequestState::Canceled
    );

    // Process #1 normally
    approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_1.pubkey(),
    )
    .expect("approve request #1 should succeed");
    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        1
    );

    // Queue now expects #2 but #2 is a tombstone — skip it
    skip_canceled_subscription_request(
        &mut svm,
        &authority,
        vault_pubkey,
        request_2.pubkey(),
        user.pubkey(),
    )
    .expect("skip tombstone for #2 should succeed");

    assert!(
        svm.get_account(&request_2.pubkey()).is_none(),
        "tombstone #2 must be closed after skip"
    );
    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        2
    );

    // Process #3
    approve_deposit_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_3.pubkey(),
    )
    .expect("approve request #3 should succeed after skip unblocked queue");

    assert_eq!(
        subscription_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_subscription_request_index,
        3
    );
    assert_eq!(
        Request::from_bytes(svm.get_account(&request_3.pubkey()).unwrap().data())
            .unwrap()
            .request_state,
        RequestState::Claimable
    );
}

// ── Failure tests ─────────────────────────────────────────────────────────────

#[test_case(true,  true,  6031, "MustUseCancelQueuedDepositRequest" ; "cancel_request_rejected_for_queued_deposit")]
#[test_case(false, true,  6029, "SubscriptionQueueOutOfOrder"        ; "skip_out_of_order_fails")]
#[test_case(false, false, 6006, "UninitializedExtension"             ; "cancel_queued_no_subscription_queue")]
fn test_cancel_and_skip_failure_cases(
    use_cancel_request: bool,
    with_queue: bool,
    expected_code: u32,
    expected_name: &str,
) {
    use async_vault_client::CancelRequestBuilder;
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        _reserve_pubkey,
        pending_vault_pubkey,
        user_token_account,
    ) = setup(with_queue);

    let request_1 = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        100_000,
    );

    if use_cancel_request {
        // cancel_request must be rejected for subscription-queue deposits
        let err = CancelRequestBuilder::new()
            .user(user.pubkey())
            .asset_mint(asset_mint.pubkey())
            .share_mint(share_mint.pubkey())
            .request(request_1.pubkey())
            .vault(vault_pubkey)
            .user_token_account(Some(user_token_account))
            .asset_pending_vault(Some(pending_vault_pubkey))
            .asset_token_program(Some(spl_token::ID))
            .user_share_account(None)
            .share_token_program(None)
            .instruction()
            .send_transaction(&mut svm, &user.pubkey(), &[&user])
            .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    } else if with_queue {
        let request_2 = create_deposit_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_token_account,
            pending_vault_pubkey,
            100_000,
        );
        // skip out of order: cancel #2 (tombstone), then try to skip #2 before #1 is processed
        cancel_queued_deposit_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_token_account,
            pending_vault_pubkey,
            request_2.pubkey(),
        )
        .expect("cancel #2 should succeed");

        // last_processed=0, expected=1, but request_2 has id=2 → SubscriptionQueueOutOfOrder
        let err = skip_canceled_subscription_request(
            &mut svm,
            &authority,
            vault_pubkey,
            request_2.pubkey(),
            user.pubkey(),
        )
        .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    } else {
        // vault has no subscription queue, so the request has no SubscriptionQueueRequest
        // extension — cancel_queued_deposit_request must return UninitializedExtension
        let err = cancel_queued_deposit_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_token_account,
            pending_vault_pubkey,
            request_1.pubkey(),
        )
        .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    }
}

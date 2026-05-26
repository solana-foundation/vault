use anchor_spl::token;
use async_vault_client::{
    extensions::redemption_queue, lite::SendTransaction, sdk::program_id, ApproveRequestBuilder,
    CancelQueuedRedemptionRequestBuilder, CreateRedeemRequestBuilder,
    InitializeRedemptionQueueBuilder, InitializeVaultBuilder as InitializeAsyncVaultBuilder,
    RejectRequestBuilder, Request, RequestArgs, RequestState, SkipCanceledQueueRequestBuilder,
    UpdateVaultNavBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, helper_mint_to, set_share_balance, set_up_async_vault,
    set_vault_total_asset_balance,
};

const NAV: u128 = 1_000_000_000;
// Pre-fund user with enough shares to create multiple redeem requests across all tests.
const USER_SHARE_BALANCE: u64 = 10_000_000;
// Asset tokens minted into the vault reserve so approve_redemption_request can settle.
// With NAV=1e9 and 9 decimals, 100_000 shares → 100_000 asset tokens per request.
const VAULT_RESERVE_BALANCE: u64 = 100_000_000_000;

// ── Test setup ────────────────────────────────────────────────────────────────

#[allow(clippy::type_complexity)]
fn setup(
    with_redemption_queue: bool,
) -> (
    LiteSVM,
    Keypair, // authority
    Keypair, // asset_mint
    Keypair, // share_mint
    Keypair, // user
    Pubkey,  // vault_pubkey
    Pubkey,  // reserve_pubkey
    Pubkey,  // pending_vault_pubkey
    Pubkey,  // user_share_account
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _payer,
        mint_authority,
        asset_mint,
        share_mint,
        user,
        _operator,
        _fee_recipient,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _fee_recipient_ata,
        user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 1_000_000_000);

    if with_redemption_queue {
        InitializeRedemptionQueueBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize_redemption_queue should succeed");
    }

    InitializeAsyncVaultBuilder::new()
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

    // Pre-fund user with shares so tests can create redeem requests immediately.
    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        USER_SHARE_BALANCE,
    );

    // Fund the vault reserve with asset tokens so approve_redemption_request can settle
    // (settle_redeem transfers from vault reserve to pending_vault).
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        VAULT_RESERVE_BALANCE,
        &token::ID,
    );
    // Sync the vault's tracked asset balance with the funded reserve so the checked_sub
    // in approve_request doesn't underflow.
    set_vault_total_asset_balance(&mut svm, vault_pubkey, VAULT_RESERVE_BALANCE);

    (
        svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_share_account,
    )
}

fn create_redeem_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    user_share_account: Pubkey,
    amount: u64,
) -> Keypair {
    let request_keypair = Keypair::new();
    CreateRedeemRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_share_account(user_share_account)
        .share_token_program(spl_token::ID)
        .args(RequestArgs {
            amount,
            operator: None,
        })
        .instruction()
        .send_transaction(svm, &user.pubkey(), &[user, &request_keypair])
        .expect("create_redeem_request should succeed");
    request_keypair
}

fn approve_redemption_request(
    svm: &mut LiteSVM,
    authority: &Keypair,
    vault_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    pending_vault_pubkey: Pubkey,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    request_pubkey: Pubkey,
) -> litesvm::types::TransactionResult {
    ApproveRequestBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .request(request_pubkey)
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .vault_token_account(reserve_pubkey)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token::ID)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

fn reject_redemption_request(
    svm: &mut LiteSVM,
    authority: &Keypair,
    user_pubkey: Pubkey,
    vault_pubkey: Pubkey,
    share_mint: Pubkey,
    asset_mint: Pubkey,
    request_pubkey: Pubkey,
    user_share_account: Pubkey,
) -> litesvm::types::TransactionResult {
    RejectRequestBuilder::new()
        .authority(authority.pubkey())
        .user(user_pubkey)
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .request(request_pubkey)
        .vault(vault_pubkey)
        .user_token_account(None)
        .asset_pending_vault(None)
        .asset_token_program(None)
        .user_share_account(Some(user_share_account))
        .share_token_program(Some(token::ID))
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

fn cancel_queued_redemption_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    user_share_account: Pubkey,
    request_pubkey: Pubkey,
) -> litesvm::types::TransactionResult {
    CancelQueuedRedemptionRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .vault(vault_pubkey)
        .request(request_pubkey)
        .user_share_account(user_share_account)
        .share_token_program(spl_token::ID)
        .instruction()
        .send_transaction(svm, &user.pubkey(), &[user])
}

fn skip_canceled_redemption_request(
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_redemption_queue_state_is_zeroed() {
    let (svm, _authority, _asset_mint, _share_mint, _user, vault_pubkey, ..) = setup(true);

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).expect("vault exists").data())
            .expect("RedemptionQueue extension should be present");

    assert_eq!(
        queue.all_time_total_redemption_requests, 0,
        "all_time_total should start at 0"
    );
    assert_eq!(
        queue.last_processed_redemption_request_index, 0,
        "last_processed should start at 0"
    );
}

#[test_case(true, false, 6004, "VaultAlreadyInitialized" ; "after_vault_init")]
#[test_case(false, true, 6005, "ExtensionAlreadyInitialized" ; "duplicate")]
fn test_initialize_redemption_queue_fails(
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
        _share_mint,
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
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize vault should succeed");
    }

    if init_extension_first {
        InitializeRedemptionQueueBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("first initialize should succeed");
        svm.expire_blockhash();
    }

    let err = InitializeRedemptionQueueBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

#[test]
fn test_create_redeem_request_increments_counter_and_sets_request_id() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_share_account,
    ) = setup(true);

    let request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_redemption_requests, 1,
        "counter should be 1 after first request"
    );
    assert_eq!(
        queue.last_processed_redemption_request_index, 0,
        "last_processed unchanged"
    );

    let id_1 = redemption_queue::get_request_state(
        svm.get_account(&request_1.pubkey()).unwrap().data(),
    )
    .expect("request 1 should have RedemptionQueueRequest extension")
    .id;
    assert_eq!(id_1, 1, "first request should have id=1");

    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_redemption_requests, 2,
        "counter should be 2 after second request"
    );
    assert_eq!(
        queue.last_processed_redemption_request_index, 0,
        "last_processed still unchanged"
    );

    let id_2 = redemption_queue::get_request_state(
        svm.get_account(&request_2.pubkey()).unwrap().data(),
    )
    .expect("request 2 should have RedemptionQueueRequest extension")
    .id;
    assert_eq!(id_2, 2, "second request should have id=2");

    let vault = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(vault.pending_async_requests, 2);

    // Approve request 1 to validate end-to-end state
    approve_redemption_request(
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

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.all_time_total_redemption_requests, 2,
        "all_time_total unchanged after approve"
    );
    assert_eq!(
        queue.last_processed_redemption_request_index, 1,
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
        user_share_account,
    ) = setup(true);

    let _request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    // Attempt to approve request 2 before request 1 — must fail
    let err = approve_redemption_request(
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
    assert_error_code(&err, 6032, "RedemptionQueueOutOfOrder");

    // Vault state must be unchanged
    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_redemption_request_index, 0,
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
        _pending_vault_pubkey,
        user_share_account,
    ) = setup(true);

    let _request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    let err = reject_redemption_request(
        &mut svm,
        &authority,
        user.pubkey(),
        vault_pubkey,
        share_mint.pubkey(),
        asset_mint.pubkey(),
        request_2.pubkey(),
        user_share_account,
    )
    .unwrap_err();
    assert_error_code(&err, 6032, "RedemptionQueueOutOfOrder");

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_redemption_request_index, 0,
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
        user_share_account,
    ) = setup(true);

    let request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_3 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    // Approve request 1
    approve_redemption_request(
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
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
        1
    );

    // Reject request 2 — shares minted back, account closed
    let share_balance_before = crate::helper_functions::get_token_account_amount(
        &svm.get_account(&user_share_account).unwrap(),
    );
    reject_redemption_request(
        &mut svm,
        &authority,
        user.pubkey(),
        vault_pubkey,
        share_mint.pubkey(),
        asset_mint.pubkey(),
        request_2.pubkey(),
        user_share_account,
    )
    .expect("reject request 2 should succeed");
    assert_eq!(
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
        2
    );
    assert_eq!(
        crate::helper_functions::get_token_account_amount(
            &svm.get_account(&user_share_account).unwrap()
        ),
        share_balance_before + 100_000,
        "rejected shares must be minted back"
    );
    assert!(
        svm.get_account(&request_2.pubkey()).is_none(),
        "rejected request account should be closed"
    );

    // Approve request 3
    approve_redemption_request(
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
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
        3
    );
    assert_eq!(
        Request::from_bytes(svm.get_account(&request_3.pubkey()).unwrap().data())
            .unwrap()
            .request_state,
        RequestState::Claimable
    );
}

// ── cancel_queued_redemption_request helpers / tests ─────────────────────────

/// Canceling the only queued request and then creating a new one should not stall the queue.
#[test]
fn test_cancel_queued_redemption_then_new_request_can_be_approved() {
    let (
        mut svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        user_share_account,
    ) = setup(true);

    let initial_share_balance = crate::helper_functions::get_token_account_amount(
        &svm.get_account(&user_share_account).unwrap(),
    );

    let request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    // Cancel request #1 via the queued cancel path
    cancel_queued_redemption_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        request_1.pubkey(),
    )
    .expect("cancel_queued_redemption_request should succeed");

    // Request account must still be open (tombstone)
    let req_data = svm
        .get_account(&request_1.pubkey())
        .expect("tombstone request account must remain open");
    assert_eq!(
        Request::from_bytes(req_data.data()).unwrap().request_state,
        RequestState::Canceled,
        "request state must be Canceled"
    );

    // Queue counter incremented; last_processed still 0
    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(queue.all_time_total_redemption_requests, 1);
    assert_eq!(queue.last_processed_redemption_request_index, 0);

    // Advance queue past the tombstone — anyone can call this
    skip_canceled_redemption_request(
        &mut svm,
        &authority,
        vault_pubkey,
        request_1.pubkey(),
        user.pubkey(),
    )
    .expect("skip_canceled_redemption_request should succeed");

    // Tombstone account must be closed
    assert!(
        svm.get_account(&request_1.pubkey()).is_none(),
        "tombstone account must be closed after skip"
    );

    let queue =
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(
        queue.last_processed_redemption_request_index, 1,
        "queue must have advanced past canceled request"
    );

    // Create request #2 (ID=2) and approve it — must succeed
    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    approve_redemption_request(
        &mut svm,
        &authority,
        vault_pubkey,
        reserve_pubkey,
        pending_vault_pubkey,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_2.pubkey(),
    )
    .expect("approve request #2 should succeed after queue unblock");

    assert_eq!(
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
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
        user_share_account,
    ) = setup(true);

    let request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_2 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );
    let request_3 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    let share_balance_before_cancel = crate::helper_functions::get_token_account_amount(
        &svm.get_account(&user_share_account).unwrap(),
    );

    // Cancel the middle request
    cancel_queued_redemption_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        request_2.pubkey(),
    )
    .expect("cancel_queued_redemption_request for #2 should succeed");

    assert_eq!(
        Request::from_bytes(svm.get_account(&request_2.pubkey()).unwrap().data())
            .unwrap()
            .request_state,
        RequestState::Canceled
    );
    assert_eq!(
        crate::helper_functions::get_token_account_amount(
            &svm.get_account(&user_share_account).unwrap()
        ),
        share_balance_before_cancel + 100_000,
        "canceled request #2 shares must be minted back"
    );

    // Process #1 normally
    approve_redemption_request(
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
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
        1
    );

    // Queue now expects #2 but #2 is a tombstone — skip it
    skip_canceled_redemption_request(
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
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
        2
    );

    // Process #3
    approve_redemption_request(
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
        redemption_queue::get_state(svm.get_account(&vault_pubkey).unwrap().data())
            .unwrap()
            .last_processed_redemption_request_index,
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

#[test_case(true,  true,  6033, "MustUseCancelQueuedRedemptionRequest" ; "cancel_request_rejected_for_queued_redeem")]
#[test_case(false, true,  6032, "RedemptionQueueOutOfOrder"             ; "skip_out_of_order_fails")]
#[test_case(false, false, 6006, "UninitializedExtension"                ; "cancel_queued_no_redemption_queue")]
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
        _pending_vault_pubkey,
        user_share_account,
    ) = setup(with_queue);

    let request_1 = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        100_000,
    );

    if use_cancel_request {
        // cancel_request must be rejected for redemption-queue redeem requests
        let err = CancelRequestBuilder::new()
            .user(user.pubkey())
            .asset_mint(asset_mint.pubkey())
            .share_mint(share_mint.pubkey())
            .request(request_1.pubkey())
            .vault(vault_pubkey)
            .user_token_account(None)
            .asset_pending_vault(None)
            .asset_token_program(None)
            .user_share_account(Some(user_share_account))
            .share_token_program(Some(spl_token::ID))
            .instruction()
            .send_transaction(&mut svm, &user.pubkey(), &[&user])
            .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    } else if with_queue {
        let request_2 = create_redeem_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_share_account,
            100_000,
        );
        // skip out of order: cancel #2 (tombstone), then try to skip #2 before #1 is processed
        cancel_queued_redemption_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_share_account,
            request_2.pubkey(),
        )
        .expect("cancel #2 should succeed");

        // last_processed=0, expected=1, but request_2 has id=2 → RedemptionQueueOutOfOrder
        let err = skip_canceled_redemption_request(
            &mut svm,
            &authority,
            vault_pubkey,
            request_2.pubkey(),
            user.pubkey(),
        )
        .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    } else {
        // vault has no redemption queue, so the request has no RedemptionQueueRequest extension
        // — cancel_queued_redemption_request must return UninitializedExtension
        let err = cancel_queued_redemption_request(
            &mut svm,
            &user,
            asset_mint.pubkey(),
            share_mint.pubkey(),
            vault_pubkey,
            user_share_account,
            request_1.pubkey(),
        )
        .unwrap_err();
        assert_error_code(&err, expected_code, expected_name);
    }
}

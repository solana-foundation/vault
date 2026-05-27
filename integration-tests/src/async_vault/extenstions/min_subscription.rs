use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    extensions::min_subscription, lite::SendTransaction, sdk::program_id,
    CreateDepositRequestBuilder, InitializeMinSubscriptionBuilder,
    InitializeVaultBuilder as InitializeAsyncVaultBuilder, RequestArgs,
    UpdateMinSubscriptionBuilder, UpdateVaultNavBuilder,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::async_helper_functions::{
    assert_error_code, get_token_account_amount, set_up_async_vault,
};

const NAV: u128 = 1_000_000_000;
const THRESHOLD: u64 = 1_000_000;

fn setup(
    threshold: Option<u64>,
) -> (
    LiteSVM,
    Keypair, // authority
    Keypair, // asset_mint
    Keypair, // share_mint
    Keypair, // user
    Pubkey,  // vault_pubkey
    Pubkey,  // pending_vault_pubkey
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
        _reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, NAV as u64);

    if let Some(t) = threshold {
        InitializeMinSubscriptionBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .threshold(t)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize_min_subscription should succeed");
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

    (
        svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        pending_vault_pubkey,
    )
}

fn initialize_min_subscription(
    svm: &mut LiteSVM,
    authority: &Keypair,
    vault_pubkey: Pubkey,
    threshold: u64,
) -> litesvm::types::TransactionResult {
    InitializeMinSubscriptionBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .threshold(threshold)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

fn update_min_subscription(
    svm: &mut LiteSVM,
    authority: &Keypair,
    vault_pubkey: Pubkey,
    threshold: u64,
) -> litesvm::types::TransactionResult {
    UpdateMinSubscriptionBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .threshold(threshold)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[authority])
}

fn create_deposit_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    pending_vault_pubkey: Pubkey,
    amount: u64,
) -> litesvm::types::TransactionResult {
    let user_token_account =
        get_associated_token_address_with_program_id(&user.pubkey(), &asset_mint, &token::ID);
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
}

#[test]
fn test_initialize_min_subscription() {
    let (svm, _authority, _asset_mint, _share_mint, _user, vault_pubkey, _pending_vault_pubkey) =
        setup(Some(THRESHOLD));

    let ext =
        min_subscription::get_state(svm.get_account(&vault_pubkey).expect("vault exists").data())
            .expect("MinSubscription should be initialized");
    assert_eq!(ext.threshold, THRESHOLD);
}

#[test]
fn test_update_min_subscription() {
    let (mut svm, authority, _asset_mint, _share_mint, _user, vault_pubkey, _pending_vault_pubkey) =
        setup(Some(THRESHOLD));

    let new_threshold = THRESHOLD * 2;

    update_min_subscription(&mut svm, &authority, vault_pubkey, new_threshold)
        .expect("update_min_subscription should succeed");

    let ext =
        min_subscription::get_state(svm.get_account(&vault_pubkey).expect("vault exists").data())
            .expect("MinSubscription should still be present");
    assert_eq!(ext.threshold, new_threshold);
}

#[test]
fn test_create_deposit_request_at_threshold() {
    let (mut svm, _authority, asset_mint, share_mint, user, vault_pubkey, pending_vault_pubkey) =
        setup(Some(THRESHOLD));

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );
    let before_balance = get_token_account_amount(
        &svm.get_account(&user_token_account)
            .expect("user ata exists"),
    );
    let pending_before = get_token_account_amount(
        &svm.get_account(&pending_vault_pubkey)
            .expect("pending vault exists"),
    );

    create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        pending_vault_pubkey,
        THRESHOLD,
    )
    .expect("deposit at threshold should succeed");

    let after_balance = get_token_account_amount(
        &svm.get_account(&user_token_account)
            .expect("user ata exists"),
    );
    let pending_after = get_token_account_amount(
        &svm.get_account(&pending_vault_pubkey)
            .expect("pending vault exists"),
    );

    assert_eq!(before_balance - after_balance, THRESHOLD);
    assert_eq!(pending_after - pending_before, THRESHOLD);
}

#[test]
fn test_create_deposit_request_below_threshold() {
    let (mut svm, _authority, asset_mint, share_mint, user, vault_pubkey, pending_vault_pubkey) =
        setup(Some(THRESHOLD));

    let err = create_deposit_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        pending_vault_pubkey,
        THRESHOLD - 1,
    )
    .unwrap_err();
    assert_error_code(&err, 6034, "SubscriptionAmountBelowMinimum");
}

#[test_case(true, false, 6004, "VaultAlreadyInitialized" ; "after_vault_init")]
#[test_case(false, true, 6005, "ExtensionAlreadyInitialized" ; "duplicate")]
fn test_initialize_min_subscription_fails(
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
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, NAV as u64);

    if init_vault_first {
        InitializeAsyncVaultBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize vault should succeed");
    }

    if init_extension_first {
        initialize_min_subscription(&mut svm, &authority, vault_pubkey, THRESHOLD)
            .expect("first initialize should succeed");
        svm.expire_blockhash();
    }

    let err =
        initialize_min_subscription(&mut svm, &authority, vault_pubkey, THRESHOLD).unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

#[test_case(false, false, 6006, "UninitializedExtension" ; "without_init")]
#[test_case(true, true, 6001, "UnauthorizedSigner" ; "wrong_authority")]
fn test_update_min_subscription_fails(
    init_extension: bool,
    use_wrong_signer: bool,
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
        user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, NAV as u64);

    if init_extension {
        initialize_min_subscription(&mut svm, &authority, vault_pubkey, THRESHOLD)
            .expect("initialize_min_subscription should succeed");
    }

    let signer: &Keypair = if use_wrong_signer { &user } else { &authority };

    let err = update_min_subscription(&mut svm, signer, vault_pubkey, THRESHOLD * 2).unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

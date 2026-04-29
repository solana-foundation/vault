use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, signature::Keypair, signer::Signer, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, cancel_request, create_ata, create_deposit_request, create_redeem_request,
    get_token_account_amount, initialize_async_vault, set_share_balance, set_up_async_vault,
    update_async_vault, update_vault_nav,
};

#[test_case(1_000_000 ; "cancel deposit request refunds user")]
#[test_case(0 ; "cancel zero amount deposit succeeds")]
#[test_case(500_000_000 ; "cancel large deposit refunds full amount")]
fn test_cancel_deposit_request(deposit_amount: u64) {
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
        Some(0),
        token::ID,
        user_amount,
        100_000_000,
    );

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    let request_keypair = Keypair::new();

    let ix = create_deposit_request(
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
    .expect("deposit request should succeed");

    let user_balance_after_deposit =
        get_token_account_amount(&svm.get_account(&user_token_account).unwrap());
    assert_eq!(user_balance_after_deposit, user_amount - deposit_amount);
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        deposit_amount
    );

    let vault_before = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    let pending_before = vault_before.pending_async_requests;

    cancel_request(
        &mut svm,
        user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_keypair.pubkey(),
        vault_pubkey,
        Some(user_token_account),
        Some(pending_vault_pubkey),
        Some(token::ID),
        None,
        None,
    )
    .expect("cancel deposit request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_token_account).unwrap()),
        user_amount,
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        0
    );

    assert!(
        svm.get_account(&request_keypair.pubkey()).is_none(),
        "Request account should be closed"
    );

    let vault_after = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(vault_after.pending_async_requests, pending_before - 1,);
}

#[test_case(true ; "wrong user cannot cancel request")]
#[test_case(false ; "paused vault rejects cancel")]
fn test_cancel_deposit_request_fails(wrong_user: bool) {
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
        operator,
        _fee_recipient,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(
        &mut svm,
        token::ID,
        Some(0),
        token::ID,
        user_amount,
        100_000_000,
    );

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

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
    .expect("deposit request should succeed");

    if !wrong_user {
        update_async_vault(
            &mut svm,
            &authority,
            share_mint.pubkey(),
            vault_pubkey,
            true,
        )
        .expect("pause should succeed");
    }

    let cancel_signer = if wrong_user {
        let attacker = Keypair::new();
        svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();
        let _ = create_ata(&mut svm, &attacker, &asset_mint.pubkey(), &token::ID);
        attacker
    } else {
        user
    };

    let cancel_user_ata = get_associated_token_address_with_program_id(
        &cancel_signer.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    let err = cancel_request(
        &mut svm,
        cancel_signer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_keypair.pubkey(),
        vault_pubkey,
        Some(cancel_user_ata),
        Some(pending_vault_pubkey),
        Some(token::ID),
        None,
        None,
    )
    .unwrap_err();

    if wrong_user {
        assert_error_code(&err, 6001, "UnauthorizedSigner");
    } else {
        assert_error_code(&err, 6003, "PausedVault");
    }
}

#[test_case(1_000_000_000 ; "cancel redeem request mints shares back")]
#[test_case(500_000_000 ; "cancel partial redeem mints correct amount")]
fn test_cancel_redeem_request(share_amount: u64) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
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
        _pending_vault_pubkey,
        _fee_recipient_ata,
        user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0, 100_000_000);

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        share_amount,
    );

    let request_keypair = Keypair::new();
    let ix = create_redeem_request(
        &mut svm,
        &user,
        &request_keypair,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        share_amount,
    )
    .expect("redeem request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap()),
        0
    );

    let vault_before = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    let pending_before = vault_before.pending_async_requests;

    let ix = cancel_request(
        &mut svm,
        user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        request_keypair.pubkey(),
        vault_pubkey,
        None,
        None,
        None,
        Some(user_share_account),
        Some(token::ID),
    )
    .expect("cancel redeem request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap()),
        share_amount,
    );

    assert!(
        svm.get_account(&request_keypair.pubkey()).is_none(),
        "Request account should be closed"
    );

    let vault_after = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(vault_after.pending_async_requests, pending_before - 1);
}

use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction},
    CancelRequestBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, signature::Keypair, signer::Signer, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_ata, create_deposit_request_ix, get_token_account_amount,
    initialize_async_vault, set_up_async_vault, update_async_vault, update_vault_nav,
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

    let mut builder = CancelRequestBuilder::new();
    builder
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(user_token_account))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID));

    let ix = builder.instruction().into_sdk_instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
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

    let mut builder = CancelRequestBuilder::new();
    builder
        .user(cancel_signer.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(cancel_user_ata))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID));

    let ix = builder.instruction().into_sdk_instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&cancel_signer.pubkey()),
        &[&cancel_signer],
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();

    if wrong_user {
        assert_error_code(&err, 6001, "UnauthorizedSigner");
    } else {
        assert_error_code(&err, 6003, "PausedVault");
    }
}

#[test]
fn test_cancel_multiple_deposit_requests() {
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

    let deposit_amount = 100_000;

    let request_1 = Keypair::new();
    let ix1 = create_deposit_request_ix(
        &user,
        &request_1,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        deposit_amount,
    );
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&user.pubkey()),
        &[&user, &request_1],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx1)
        .expect("first deposit should succeed");

    let request_2 = Keypair::new();
    let ix2 = create_deposit_request_ix(
        &user,
        &request_2,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_token_account,
        pending_vault_pubkey,
        deposit_amount,
    );
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&user.pubkey()),
        &[&user, &request_2],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2)
        .expect("second deposit should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        deposit_amount * 2
    );

    let mut builder = CancelRequestBuilder::new();
    builder
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_1.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(user_token_account))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID));

    let ix = builder.instruction().into_sdk_instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("cancel first request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        deposit_amount
    );
    assert!(svm.get_account(&request_1.pubkey()).is_none());
    assert!(svm.get_account(&request_2.pubkey()).is_some());

    let mut builder2 = CancelRequestBuilder::new();
    builder2
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_2.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(user_token_account))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID));

    let ix2 = builder2.instruction().into_sdk_instruction();
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2)
        .expect("cancel second request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_token_account).unwrap()),
        user_amount
    );

    let vault = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    assert_eq!(vault.pending_async_requests, 0);
}

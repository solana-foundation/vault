use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::sdk::program_id;
use litesvm::LiteSVM;
use solana_sdk::{signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_ata, get_token_account_amount, helper_mint_to,
    initialize_async_vault, set_up_async_vault, update_async_vault, update_vault_nav,
    withdraw_assets,
};

#[test_case(1_000_000, 500_000 ; "withdraw partial amount")]
#[test_case(1_000_000, 1_000_000 ; "withdraw full amount")]
#[test_case(1_000_000, 1 ; "withdraw minimum amount")]
fn test_withdraw_assets_success(deposit_amount: u64, withdraw_amount: u64) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let user_amount = 10_000_000;
    let (
        authority,
        _payer,
        mint_authority,
        asset_mint,
        share_mint,
        _user,
        _operator,
        _fee_recipient,
        reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
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
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        deposit_amount,
        &token::ID,
    );

    let recipient = Keypair::new();
    svm.airdrop(&recipient.pubkey(), 1_000_000_000).unwrap();
    let recipient_ata = create_ata(&mut svm, &recipient, &asset_mint.pubkey(), &token::ID);

    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_before, deposit_amount);

    withdraw_assets(
        &mut svm,
        &authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        reserve_pubkey,
        recipient_ata,
        token::ID,
        withdraw_amount,
    )
    .expect("withdraw assets should succeed");

    let reserve_after = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_after, deposit_amount - withdraw_amount);

    let recipient_balance = get_token_account_amount(&svm.get_account(&recipient_ata).unwrap());
    assert_eq!(recipient_balance, withdraw_amount);
}

#[test_case(true, false ; "unauthorized signer")]
#[test_case(false, true ; "paused vault")]
fn test_withdraw_assets_fails(use_wrong_signer: bool, pause_vault: bool) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
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
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(
        &mut svm,
        token::ID,
        None,
        token::ID,
        10_000_000,
        100_000_000,
    );

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        1_000_000,
        &token::ID,
    );

    let recipient = Keypair::new();
    svm.airdrop(&recipient.pubkey(), 1_000_000_000).unwrap();
    let recipient_ata = create_ata(&mut svm, &recipient, &asset_mint.pubkey(), &token::ID);

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

    let signer = if use_wrong_signer { &user } else { &authority };

    let expected_error_code = if use_wrong_signer { 6001 } else { 6003 };

    let err = withdraw_assets(
        &mut svm,
        signer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        reserve_pubkey,
        recipient_ata,
        token::ID,
        500_000,
    )
    .unwrap_err();

    assert_error_code(&err, expected_error_code, "");
}

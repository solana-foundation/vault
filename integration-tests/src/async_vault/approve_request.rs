use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, ApproveRequestBuilder, CreateDepositRequestBuilder,
    CreateRedeemRequestBuilder, InitializeVaultBuilder as InitializeAsyncVaultBuilder, Request,
    RequestArgs, RequestState, UpdateVaultBuilder as UpdateVaultAsyncBuilder,
    UpdateVaultNavBuilder, Vault,
};
use borsh::BorshSerialize;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use test_case::test_case;

use crate::async_helper_functions::{
    assert_error_code, get_token_account_amount, helper_mint_to, set_share_balance,
    set_up_async_vault, set_vault_total_asset_balance,
};

fn setup(
    svm: &mut LiteSVM,
    nav: u128,
) -> (
    Keypair, // authority
    Keypair, // mint_authority
    Keypair, // asset_mint
    Keypair, // share_mint
    Keypair, // user
    Pubkey,  // reserve_pubkey
    Pubkey,  // vault_pubkey
    Pubkey,  // pending_vault_pubkey
    Pubkey,  // user_asset_account
    Pubkey,  // user_share_account
) {
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
    ) = set_up_async_vault(svm, token::ID, None, token::ID, 1_000_000_000);

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");

    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(nav)
        .instruction()
        .send_transaction(svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

    let user_asset_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    (
        authority,
        mint_authority,
        asset_mint,
        share_mint,
        user,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        user_asset_account,
        user_share_account,
    )
}

#[test_case(200, 1_000_000, 5_000_000_000_000 ; "nav=200_10-9")]
#[test_case(200_000_000_000, 1_000_000, 5_000 ; "nav=200")]
fn test_approve_deposit_request_success(
    nav: u128,
    deposit_amount: u64,
    expected_deposit_shares: u64,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _mint_authority,
        asset_mint,
        share_mint,
        user,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        user_asset_account,
        _user_share_account,
    ) = setup(&mut svm, nav);

    let request_keypair = Keypair::new();
    CreateDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(user_asset_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: deposit_amount,
            operator: None,
        })
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &request_keypair])
        .expect("create deposit request should succeed");

    let pending_before = get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap());
    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());

    ApproveRequestBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .request(request_keypair.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault_token_account(reserve_pubkey)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token::ID)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("approve_request should succeed");

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
    assert_eq!(request_after.price, nav);
    assert_eq!(
        request_after.amount, expected_deposit_shares,
        "request.amount should be updated to claimable shares"
    );

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

#[test_case(200, 5_000_000_000_000, 1_000_000 ; "nav=200_10-9")]
#[test_case(200_000_000_000, 5_000, 1_000_000 ; "nav=200")]
fn test_approve_redeem_request_success(nav: u128, redeem_amount: u64, expected_redeem_assets: u64) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        mint_authority,
        asset_mint,
        share_mint,
        user,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _user_asset_account,
        user_share_account,
    ) = setup(&mut svm, nav);

    // Fund reserve with assets to cover the redeem payout
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        expected_redeem_assets,
        &token::ID,
    );

    set_vault_total_asset_balance(&mut svm, vault_pubkey, expected_redeem_assets);

    // Give the user shares to redeem
    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        redeem_amount,
    );

    let request_keypair = Keypair::new();
    CreateRedeemRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_share_account(user_share_account)
        .share_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: redeem_amount,
            operator: None,
        })
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &request_keypair])
        .expect("create redeem request should succeed");

    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let pending_before = get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap());

    ApproveRequestBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .request(request_keypair.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault_token_account(reserve_pubkey)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token::ID)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("approve_request should succeed");

    let vault_after = Vault::from_bytes(
        svm.get_account(&vault_pubkey)
            .expect("vault should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(vault_after.pending_async_requests, 0);
    assert_eq!(
        vault_after.total_asset_balance, 0,
        "total_asset_balance should be decremented by redeemed assets"
    );

    let request_after = Request::from_bytes(
        svm.get_account(&request_keypair.pubkey())
            .expect("request should exist")
            .data(),
    )
    .unwrap();
    assert_eq!(request_after.request_state, RequestState::Claimable);
    assert_eq!(request_after.price, nav);
    assert_eq!(
        request_after.amount, expected_redeem_assets,
        "request.amount should be updated to claimable assets"
    );

    assert_eq!(
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap()),
        reserve_before - expected_redeem_assets,
        "reserve should be drained by redeemed assets"
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        pending_before + expected_redeem_assets,
        "pending_vault should receive the redeemed assets"
    );
}

#[test_case(false, true, false, 1_000_000, 6001 ; "unauthorized signer")]
#[test_case(true, false, false, 1_000_000, 6003 ; "paused vault")]
#[test_case(false, false, true, 1_000_000, 6021 ; "request not in pending state")]
#[test_case(false, false, false, 1_000_000, 6029 ; "nav not set")]
fn test_approve_request_fails(
    pause_vault: bool,
    use_wrong_signer: bool,
    override_to_claimable: bool,
    deposit_amount: u64,
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
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, user_amount);
    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token::ID,
    );

    let request_keypair = Keypair::new();
    CreateDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(user_token_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: deposit_amount,
            operator: None,
        })
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &request_keypair])
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
        UpdateVaultAsyncBuilder::new()
            .authority(authority.pubkey())
            .share_mint(share_mint.pubkey())
            .paused(true)
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
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

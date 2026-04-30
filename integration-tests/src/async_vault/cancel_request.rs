use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, CancelRequestBuilder, CreateDepositRequestBuilder,
    CreateRedeemRequestBuilder, InitializeVaultBuilder as InitializeAsyncVaultBuilder, RequestArgs,
    UpdateVaultBuilder as UpdateVaultAsyncBuilder, UpdateVaultNavBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_ata, get_token_account_amount, set_share_balance, set_up_async_vault,
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

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");
    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(100)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

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

    let user_pubkey = user.pubkey();
    CancelRequestBuilder::new()
        .user(user_pubkey)
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(user_token_account))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID))
        .user_share_account(None)
        .share_token_program(None)
        .instruction()
        .send_transaction(&mut svm, &user_pubkey, &[user])
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

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");
    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(100)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

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
            amount: 1_000_000,
            operator: None,
        })
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &request_keypair])
        .expect("deposit request should succeed");

    if !wrong_user {
        UpdateVaultAsyncBuilder::new()
            .authority(authority.pubkey())
            .share_mint(share_mint.pubkey())
            .paused(true)
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
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

    let cancel_signer_pubkey = cancel_signer.pubkey();
    let err = CancelRequestBuilder::new()
        .user(cancel_signer_pubkey)
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(Some(cancel_user_ata))
        .asset_pending_vault(Some(pending_vault_pubkey))
        .asset_token_program(Some(token::ID))
        .user_share_account(None)
        .share_token_program(None)
        .instruction()
        .send_transaction(&mut svm, &cancel_signer_pubkey, &[cancel_signer])
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

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");
    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(100)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        share_amount,
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
            amount: share_amount,
            operator: None,
        })
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &request_keypair])
        .expect("redeem request should succeed");

    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap()),
        0
    );

    let vault_before = Vault::from_bytes(svm.get_account(&vault_pubkey).unwrap().data()).unwrap();
    let pending_before = vault_before.pending_async_requests;

    let user_pubkey = user.pubkey();
    CancelRequestBuilder::new()
        .user(user_pubkey)
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(None)
        .asset_pending_vault(None)
        .asset_token_program(None)
        .user_share_account(Some(user_share_account))
        .share_token_program(Some(token::ID))
        .instruction()
        .send_transaction(&mut svm, &user_pubkey, &[user])
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

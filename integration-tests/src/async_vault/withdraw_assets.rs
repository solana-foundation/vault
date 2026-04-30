use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    sdk::program_id, InitializeVaultBuilder as InitializeAsyncVaultBuilder,
    UpdateVaultBuilder as UpdateVaultAsyncBuilder, UpdateVaultNavBuilder, WithdrawAssetsBuilder,
    lite::SendTransaction,
};
use litesvm::LiteSVM;
use solana_sdk::{signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_ata, get_token_account_amount, helper_mint_to, set_up_async_vault,
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

    WithdrawAssetsBuilder::new()
        .authority(authority.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .vault_token_account(reserve_pubkey)
        .recipient_token_account(recipient_ata)
        .asset_token_program(token::ID)
        .amount(withdraw_amount)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
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
        UpdateVaultAsyncBuilder::new()
            .authority(authority.pubkey())
            .share_mint(share_mint.pubkey())
            .paused(true)
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("pause vault should succeed");
    }

    let signer = if use_wrong_signer { &user } else { &authority };

    let expected_error_code = if use_wrong_signer { 6001 } else { 6003 };

    let err = WithdrawAssetsBuilder::new()
        .authority(signer.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .vault_token_account(reserve_pubkey)
        .recipient_token_account(recipient_ata)
        .asset_token_program(token::ID)
        .amount(500_000)
        .instruction()
        .send_transaction(&mut svm, &signer.pubkey(), &[signer])
        .unwrap_err();

    assert_error_code(&err, expected_error_code, "");
}

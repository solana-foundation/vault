use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, CreateDepositRequestBuilder,
    InitializeVaultBuilder as InitializeAsyncVaultBuilder, Request, RequestArgs,
    SetOperatorBuilder, UpdateVaultNavBuilder,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};

use crate::async_helper_functions::set_up_async_vault;

#[test]
fn test_set_operator_succeeds() {
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
        operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _pending_shares_vault_pubkey,
        _fee_recipient_ata,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 1_000_000_000);

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
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
        .expect("create deposit request should succeed");

    SetOperatorBuilder::new()
        .user(user.pubkey())
        .operator(operator.pubkey())
        .request(request_keypair.pubkey())
        .instruction()
        .send_transaction(&mut svm, &user.pubkey(), &[&user, &operator])
        .expect("set operator should succeed");

    let request_account = svm
        .get_account(&request_keypair.pubkey())
        .expect("Request account should exist");
    let request_data = Request::from_bytes(request_account.data()).unwrap();
    assert_eq!(request_data.operator, Some(operator.pubkey()));
}

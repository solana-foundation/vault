use anchor_spl::token;
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction},
    CreateDepositRequestBuilder, Request, RequestState, RequestType, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    create_async_vault, create_ata, create_mint, get_token_account_amount, helper_mint_to,
    initialize_async_vault, update_vault_nav, PENDING_VAULT_SEED, REQUEST_SEED,
    RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
};

#[test_case(1_000_000, false ; "deposit request succeeds")]
#[test_case(1_000_000, true ; "deposit with operator succeeds")]
#[test_case(0, false ; "zero amount deposit succeeds")]
fn test_create_deposit_request(deposit_amount: u64, with_operator: bool) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let user = Keypair::new();
    let operator = Keypair::new();
    let fee_recipient = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&operator.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint, &token::ID);
    create_mint(&mut svm, &mint_authority, &share_mint, &token::ID);

    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[RESERVE_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );
    let (pending_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_VAULT_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );

    create_async_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        fee_recipient.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
        100_000_000,
        true,
        true,
        token::ID,
        token::ID,
    )
    .expect("vault creation should succeed");

    let _ = initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey);

    let user_token_account = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let fee_recipient_ata = create_ata(
        &mut svm,
        &fee_recipient,
        &asset_mint.pubkey(),
        &spl_token::ID,
    );
    let user_amount = 1_000_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_token_account,
        &mint_authority,
        user_amount,
        &token::ID,
    );

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    let _ = update_vault_nav(&mut svm, &authority, share_mint.pubkey(), vault_pubkey, 100);

    let (request_pubkey, _) = Pubkey::find_program_address(
        &[
            REQUEST_SEED,
            share_mint.pubkey().as_ref(),
            vault_config.request_counter.to_be_bytes().as_ref(),
        ],
        &program_id(),
    );

    // Verify initial state
    assert_eq!(
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_token_account).unwrap()),
        user_amount
    );

    let mut builder = CreateDepositRequestBuilder::new();
    builder
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_pubkey)
        .vault(vault_pubkey)
        .user_token_account(user_token_account)
        .pending_vault(pending_vault_pubkey)
        .fee_recipient(fee_recipient_ata)
        .asset_token_program(spl_token::ID)
        .amount(deposit_amount);

    if with_operator {
        builder.operator(operator.pubkey());
    }

    let ix = builder.instruction().into_sdk_instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);

    result.expect("create deposit request should succeed");

    let request_account = svm
        .get_account(&request_pubkey)
        .expect("Request account should exist");
    let request_data = Request::from_bytes(request_account.data()).unwrap();

    assert_eq!(request_data.vault, vault_pubkey);
    assert_eq!(request_data.request_type, RequestType::Deposit);
    assert_eq!(request_data.request_state, RequestState::Pending);
    assert_eq!(request_data.owner, user.pubkey());
    assert_eq!(request_data.amount, deposit_amount);
    assert_eq!(request_data.price, 100);
    assert_eq!(request_data.asset_mint_address, asset_mint.pubkey());
    assert_eq!(request_data.nav_update_version, 1);

    if with_operator {
        assert_eq!(request_data.operator, Some(operator.pubkey()));
    } else {
        assert_eq!(request_data.operator, None);
    }

    // Verify end state
    assert_eq!(
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        deposit_amount
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_token_account).unwrap()),
        user_amount - deposit_amount
    );
}

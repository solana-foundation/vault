use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id, token, token_2022,
};
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction},
    CreateDepositRequestBuilder, Request, RequestArgs, RequestState, RequestType,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, clock::Clock, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_async_vault, create_ata, create_deposit_request_ix, create_mint,
    get_token_account_amount, helper_mint_to, initialize_async_vault, set_up_async_vault,
    update_vault_nav, PENDING_VAULT_SEED, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
};

#[test_case(1_000_000, false ; "deposit request succeeds")]
#[test_case(1_000_000, true ; "deposit with operator succeeds")]
#[test_case(0, false ; "zero amount deposit succeeds")]
fn test_create_deposit_request(deposit_amount: u64, with_operator: bool) {
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
        None,
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
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(user_token_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID);

    if with_operator {
        builder.args(RequestArgs {
            amount: deposit_amount,
            operator: Some(operator.pubkey()),
        });
    } else {
        builder.args(RequestArgs {
            amount: deposit_amount,
            operator: None,
        });
    }

    let mut ix = builder.instruction().into_sdk_instruction();
    ix.accounts.push(solana_sdk::instruction::AccountMeta::new(
        fee_recipient_ata,
        false,
    ));
    for meta in &mut ix.accounts {
        if meta.pubkey == request_keypair.pubkey() {
            meta.is_signer = true;
        }
    }
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);

    result.expect("create deposit request should succeed");

    let request_account = svm
        .get_account(&request_keypair.pubkey())
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
    assert_eq!(request_data.fee, 0);

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

#[test]
fn test_multiple_deposit_requests_with_unique_keypairs() {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let user = Keypair::new();
    let fee_recipient = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

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
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_token_account,
        &mint_authority,
        5_000_000_000,
        &token::ID,
    );

    let _ = update_vault_nav(&mut svm, &authority, vault_pubkey, 100);

    let deposit_amount = 1_000_000;

    // First deposit request with a unique keypair
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
        .expect("first deposit request should succeed");

    // Second deposit request with a different unique keypair
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
        .expect("second deposit request should succeed");

    // Both request accounts exist independently
    let req1_data = Request::from_bytes(
        svm.get_account(&request_1.pubkey())
            .expect("request 1 should exist")
            .data
            .as_slice(),
    )
    .unwrap();
    let req2_data = Request::from_bytes(
        svm.get_account(&request_2.pubkey())
            .expect("request 2 should exist")
            .data
            .as_slice(),
    )
    .unwrap();

    assert_ne!(request_1.pubkey(), request_2.pubkey());
    assert_eq!(req1_data.vault, vault_pubkey);
    assert_eq!(req2_data.vault, vault_pubkey);
    assert_eq!(req1_data.amount, deposit_amount);
    assert_eq!(req2_data.amount, deposit_amount);
    assert_eq!(req1_data.request_state, RequestState::Pending);
    assert_eq!(req2_data.request_state, RequestState::Pending);

    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        deposit_amount * 2
    );
}

#[test_case(Some(1), 6017 ; "deposit_request_with_nonzero_transfer_fee_fails")]
fn test_create_deposit_request_fails(asset_transfer_fee: Option<u16>, expected_error_code: u32) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let user_amount = 1_000_000_000;
    let (
        authority,
        _payer,
        mint_authority,
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
        token_2022::ID,
        Some(0), // Must start with TransferFee of 0 to create the Vault
        token::ID,
        user_amount,
        100_000_000,
    );

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    // Update TransferFee to nonzero after vault creation
    if let Some(fee_bps) = asset_transfer_fee {
        let set_fee_ix =
            token_2022::spl_token_2022::extension::transfer_fee::instruction::set_transfer_fee(
                &token_2022::ID,
                &asset_mint.pubkey(),
                &mint_authority.pubkey(),
                &[],
                fee_bps,
                u64::MAX,
            )
            .unwrap();

        let tx = Transaction::new_signed_with_payer(
            &[set_fee_ix],
            Some(&mint_authority.pubkey()),
            &[&mint_authority],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx)
            .expect("set_transfer_fee should succeed");

        // Advance clock by 2 epoch to ensure TransferFeeconfig change takes effect.
        let mut clock = svm.get_sysvar::<Clock>();
        clock.epoch += 2;
        svm.set_sysvar(&clock);
    }

    let user_token_account = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &asset_mint.pubkey(),
        &token_2022::ID,
    );
    let request_keypair = Keypair::new();

    let ix = CreateDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(user_token_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(token_2022::ID)
        .args(RequestArgs {
            amount: user_amount,
            operator: None,
        })
        .instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[user, request_keypair],
        blockhash,
    );

    let res = svm.send_transaction(tx).err().unwrap();
    assert_error_code(&res, expected_error_code, "Incorrect error code");
}

use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id, token, token_2022,
};
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, CreateDepositRequestBuilder,
    InitializeVaultBuilder as InitializeAsyncVaultBuilder, Request, RequestArgs, RequestState,
    RequestType, UpdateVaultNavBuilder,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, clock::Clock, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, get_token_account_amount, set_up_async_vault};

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
        Some(0),
        token::ID,
        user_amount,
        100_000_000,
    );

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

    let mut ix = builder.instruction();
    ix.accounts.push(solana_sdk::instruction::AccountMeta::new(
        fee_recipient_ata,
        false,
    ));

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

#[test_case(Some(1), 6021 ; "deposit_request_with_nonzero_transfer_fee_fails")]
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

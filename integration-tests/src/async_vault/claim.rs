use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use async_vault_client::{
    sdk::program_id, CreateDepositRequestBuilder, CreateRedeemRequestBuilder, RequestArgs, Vault,
};
use borsh::BorshSerialize;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    approve_request, assert_error_code, claim_request, get_mint_supply, get_token_account_amount,
    helper_mint_to, initialize_async_vault, set_share_balance, set_up_async_vault,
    update_async_vault, update_vault_nav,
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
    Keypair, // operator
    Pubkey,  // reserve_pubkey (vault_token_account)
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
        operator,
        _fee_recipient,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _fee_recipient_ata,
        user_share_account,
    ) = set_up_async_vault(svm, token::ID, None, token::ID, 1_000_000_000, 100_000_000);

    initialize_async_vault(svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");

    update_vault_nav(svm, &authority, vault_pubkey, nav).expect("update nav should succeed");

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
        operator,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        user_asset_account,
        user_share_account,
    )
}

#[test_case(false, 1_000_000, 1_000_000, 1_000_000_000 ; "owner claims deposit")]
#[test_case(true,  1_000_000, 1_000_000, 1_000_000_000 ; "operator claims deposit")]
fn test_claim_deposit_success(
    use_operator: bool,
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
        operator,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        _user_asset_account,
        user_share_account,
    ) = setup(&mut svm, nav);

    let request_keypair = Keypair::new();
    let operator_pubkey = use_operator.then_some(operator.pubkey());

    let ix = CreateDepositRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_token_account(_user_asset_account)
        .pending_vault(pending_vault_pubkey)
        .asset_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: deposit_amount,
            operator: operator_pubkey,
        })
        .instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("create deposit request should succeed");

    // Approve: assets move pending → reserve, request.amount set to shares
    approve_request(
        &mut svm,
        &authority,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        token::ID,
    )
    .expect("approve_request should succeed");

    // Snapshot balances after approve (before claim)
    let user_shares_before =
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap());
    let share_supply_before = get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());

    let claimer = if use_operator { &operator } else { &user };
    claim_request(
        &mut svm,
        claimer,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        None,
        Some(user_share_account),
        None,
        spl_token::ID,
        Some(spl_token::ID),
    )
    .expect("claim_request should succeed");

    assert!(
        svm.get_account(&request_keypair.pubkey()).is_none(),
        "request account should be closed after claim"
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap()),
        user_shares_before + expected_deposit_shares,
        "user should receive minted shares"
    );
    assert_eq!(
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap()),
        share_supply_before + expected_deposit_shares,
        "share mint supply should increase by minted shares"
    );
}

#[test_case(false, 1_000_000, 1_000_000_000, 1_000_000 ; "owner claims redeem")]
#[test_case(true,  1_000_000, 1_000_000_000, 1_000_000 ; "operator claims redeem")]
fn test_claim_redeem_success(
    use_operator: bool,
    nav: u128,
    redeem_amount: u64,
    expected_redeem_assets: u64,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        mint_authority,
        asset_mint,
        share_mint,
        user,
        operator,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        user_asset_account,
        user_share_account,
    ) = setup(&mut svm, nav);

    // Fund the reserve so approve can transfer out
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        expected_redeem_assets,
        &token::ID,
    );

    // Set vault.total_asset_balance to match the funded reserve
    {
        let mut account = svm.get_account(&vault_pubkey).unwrap();
        let mut vault = Vault::from_bytes(account.data()).unwrap();
        vault.total_asset_balance = expected_redeem_assets;
        let mut buf = Vec::new();
        vault.serialize(&mut buf).unwrap();
        account.data = buf;
        svm.set_account(vault_pubkey, account).unwrap();
    }

    // Give the user shares to redeem
    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        redeem_amount,
    );

    let request_keypair = Keypair::new();
    let operator_pubkey = use_operator.then_some(operator.pubkey());

    let ix = CreateRedeemRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_share_account(user_share_account)
        .share_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: redeem_amount,
            operator: operator_pubkey,
        })
        .instruction();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("create redeem request should succeed");

    // Approve: assets move reserve → pending_vault, request.amount set to assets
    approve_request(
        &mut svm,
        &authority,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        token::ID,
    )
    .expect("approve_request should succeed");

    // Snapshot balances after approve (before claim)
    let pending_vault_before =
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap());
    let user_assets_before =
        get_token_account_amount(&svm.get_account(&user_asset_account).unwrap());

    let claimer = if use_operator { &operator } else { &user };
    claim_request(
        &mut svm,
        claimer,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        Some(pending_vault_pubkey),
        None,
        Some(user_asset_account),
        spl_token::ID,
        None,
    )
    .expect("claim_request should succeed");

    assert!(
        svm.get_account(&request_keypair.pubkey()).is_none(),
        "request account should be closed after claim"
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_asset_account).unwrap()),
        user_assets_before + expected_redeem_assets,
        "user should receive assets"
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_vault_pubkey).unwrap()),
        pending_vault_before - expected_redeem_assets,
        "pending_vault should be drained by claim"
    );
}

#[test_case(true,  false, false, 1_000_000, 6001 ; "unauthorized signer")]
#[test_case(false, true,  false, 1_000_000, 6003 ; "paused vault")]
#[test_case(false, false, true,  1_000_000, 6022 ; "request not claimable")]
fn test_claim_fails(
    use_wrong_signer: bool,
    pause_vault: bool,
    skip_approve: bool,
    deposit_amount: u64,
    expected_error_code: u32,
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
        _operator,
        reserve_pubkey,
        vault_pubkey,
        pending_vault_pubkey,
        user_asset_account,
        user_share_account,
    ) = setup(&mut svm, 1_000_000);

    let request_keypair = Keypair::new();
    let ix = CreateDepositRequestBuilder::new()
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
        .instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &request_keypair],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("create deposit request should succeed");

    if !skip_approve {
        approve_request(
            &mut svm,
            &authority,
            vault_pubkey,
            request_keypair.pubkey(),
            asset_mint.pubkey(),
            share_mint.pubkey(),
            reserve_pubkey,
            pending_vault_pubkey,
            token::ID,
        )
        .expect("approve_request should succeed");
    }

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

    let wrong_signer = Keypair::new();
    svm.airdrop(&wrong_signer.pubkey(), 1_000_000_000).unwrap();
    let claimer = if use_wrong_signer {
        &wrong_signer
    } else {
        &user
    };

    let result = claim_request(
        &mut svm,
        claimer,
        vault_pubkey,
        request_keypair.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        None,
        Some(user_share_account),
        None,
        spl_token::ID,
        Some(spl_token::ID),
    );

    assert!(result.is_err(), "claim should fail");
    assert_error_code(&result.unwrap_err(), expected_error_code, "");
}

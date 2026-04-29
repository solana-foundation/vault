use anchor_spl::token;
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction},
    CreateRedeemRequestBuilder, Request, RequestArgs, RequestState, RequestType,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, get_token_account_amount, initialize_async_vault, set_up_async_vault,
    update_vault_nav,
};

fn set_share_balance(
    svm: &mut LiteSVM,
    user_share_account: &Pubkey,
    share_mint: &Pubkey,
    amount: u64,
) {
    let mut acct = svm.get_account(user_share_account).unwrap();
    let mut token_state = spl_token::state::Account::unpack(&acct.data).unwrap();
    token_state.amount = amount;
    spl_token::state::Account::pack(token_state, &mut acct.data).unwrap();
    svm.set_account(*user_share_account, acct).unwrap();

    let mut mint_acct = svm.get_account(share_mint).unwrap();
    let mut mint_state = spl_token::state::Mint::unpack(&mint_acct.data).unwrap();
    mint_state.supply = amount;
    spl_token::state::Mint::pack(mint_state, &mut mint_acct.data).unwrap();
    svm.set_account(*share_mint, mint_acct).unwrap();
}

#[test_case(1_000_000_000, false, None ; "redeem request succeeds")]
#[test_case(1_000_000_000, true, None ; "redeem with operator succeeds")]
#[test_case(0, false, Some((6011, "InsufficientRedeemAmount")) ; "zero amount fails")]
#[test_case(1_000, false, Some((6031, "ZeroAssets")) ; "zero assets fails")]
fn test_create_redeem_request(
    share_amount: u64,
    with_operator: bool,
    expected_error: Option<(u32, &str)>,
) {
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
        _pending_vault_pubkey,
        pending_shares_vault_pubkey,
        _fee_recipient_ata,
        user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, token::ID, 0, 100_000_000);

    initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
        .expect("initialize vault should succeed");
    update_vault_nav(&mut svm, &authority, vault_pubkey, 100).expect("update nav should succeed");

    if share_amount > 0 {
        set_share_balance(
            &mut svm,
            &user_share_account,
            &share_mint.pubkey(),
            share_amount,
        );
    }

    let request_keypair = Keypair::new();

    let operator_pubkey = if with_operator {
        Some(operator.pubkey())
    } else {
        None
    };

    let mut builder = CreateRedeemRequestBuilder::new();
    builder
        .user(user.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_share_account(user_share_account)
        .pending_shares_vault(pending_shares_vault_pubkey)
        .share_token_program(spl_token::ID)
        .args(RequestArgs {
            amount: share_amount,
            operator: operator_pubkey,
        });

    let mut ix = builder.instruction().into_sdk_instruction();
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

    if let Some((error_code, error_name)) = expected_error {
        assert!(result.is_err(), "redeem request should fail");
        assert_error_code(&result.unwrap_err(), error_code, error_name);
        return;
    }

    result.expect("create redeem request should succeed");

    let request_account = svm
        .get_account(&request_keypair.pubkey())
        .expect("Request account should exist");
    let request_data = Request::from_bytes(request_account.data()).unwrap();

    assert_eq!(request_data.vault, vault_pubkey);
    assert_eq!(request_data.request_type, RequestType::Redeem);
    assert_eq!(request_data.request_state, RequestState::Pending);
    assert_eq!(request_data.owner, user.pubkey());
    assert_eq!(request_data.amount, share_amount);
    assert_eq!(request_data.price, 100);
    assert_eq!(request_data.asset_mint_address, asset_mint.pubkey());
    assert_eq!(request_data.nav_update_version, 1);
    assert_eq!(request_data.fee, 0);
    assert_eq!(request_data.remaining_amount, 100);
    assert_eq!(request_data.operator, operator_pubkey);

    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_account).unwrap()),
        0
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&pending_shares_vault_pubkey).unwrap()),
        share_amount
    );
}

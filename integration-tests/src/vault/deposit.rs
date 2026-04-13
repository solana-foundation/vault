use anchor_spl::{
    token::{self},
    token_2022,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::Account as TokenAccount;
use test_case::test_case;
use vault_client::{sdk::program_id, FeeType, Pubkey};

use crate::vault::helper_functions::{
    assert_error_code, create_ata, create_mint, create_mint_with_transfer_fee, deposit, get_fee,
    get_mint_supply, get_token_account_amount, get_vault_asset_balance, helper_mint_to,
    recv_amount_from_params, set_up_vault,
};

#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token::ID,
    token::ID;
    "Deposit (SPL Token asset, SPL Token share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token_2022::ID,
    token::ID;
    "Deposit (Token 2022 asset, SPL Token share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token::ID,
    token_2022::ID;
    "Deposit (SPL Token asset, Token 2022 share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token_2022::ID,
    token_2022::ID;
    "Deposit (Token 2022 asset, Token 2022 share)"
)]
fn test_deposit_vault(deposit_fee: Option<FeeType>, asset_program: Pubkey, share_program: Pubkey) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    let mut asset_transfer_fee_bps: u16 = 0;
    let mut asset_transfer_fee_max: u64 = 0;

    if asset_program == token::ID {
        create_mint(&mut svm, &mint_authority, &asset_mint);
    } else {
        asset_transfer_fee_bps = 10;
        asset_transfer_fee_max = 1000;
        create_mint_with_transfer_fee(
            &mut svm,
            &mint_authority,
            &asset_mint,
            asset_transfer_fee_bps,
            asset_transfer_fee_max,
        );
    }

    if share_program == token::ID {
        create_mint(&mut svm, &mint_authority, &share_mint);
    } else {
        create_mint_with_transfer_fee(&mut svm, &mint_authority, &share_mint, 10, 1000);
    }

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        asset_program,
        share_program,
        deposit_fee.clone(),
        None,
    );

    let fee_recipient_ata = create_ata(
        &mut svm,
        &fee_recipient,
        &asset_mint.pubkey(),
        &asset_program,
    );
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &asset_program);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &share_program);

    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &asset_program,
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
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap()),
        user_asset_amount
    );
    assert_eq!(
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap()),
        0
    );
    assert_eq!(get_vault_asset_balance(&svm, &vault_pubkey), 0);

    let deposit_amount = 500_000;
    let result = deposit(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        deposit_amount,
        0, // no slippage protection
        asset_program,
        share_program,
        hook_client::HOOK_PROGRAM_ID,
        None,
        None,
    );

    assert!(result.is_ok(), "Unexpected result for test case");

    // Calculate expected values accounting for asset transfer fees
    let fee = get_fee(deposit_fee.clone(), deposit_amount);
    let deposit_net = deposit_amount.checked_sub(fee).expect("overflow");

    let fee_received = recv_amount_from_params(fee, asset_transfer_fee_bps, asset_transfer_fee_max);
    let deposit_net_received =
        recv_amount_from_params(deposit_net, asset_transfer_fee_bps, asset_transfer_fee_max);

    // Assert post-deposit state
    let fee_recipient_balance_after =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(fee_recipient_balance_after, fee_received);

    let user_asset_balance_after =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(deposit_amount)
            .expect("overflow")
    );

    let user_share_balance_after =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(user_share_balance_after, deposit_net_received);

    let reserve_balance_after =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_balance_after, deposit_net_received);

    let share_supply = get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(share_supply, deposit_net_received);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_net_received);
}

#[test]
fn test_deposit_slippage_protection() {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();

    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token::ID,
        token::ID,
        Some(FeeType::Percentage { bps: 100 }),
        None,
    );

    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID,
    );

    let deposit_amount = 500_000;
    let fee = get_fee(Some(FeeType::Percentage { bps: 100 }), deposit_amount);
    let expected_shares = deposit_amount.checked_sub(fee).unwrap();

    // force slippage failure: ask for more shares than can be minted
    let min_shares = expected_shares + 1;

    let result = deposit(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        deposit_amount,
        min_shares,
        token::ID,
        token::ID,
        hook_client::HOOK_PROGRAM_ID,
        None,
        None,
    );

    assert_error_code(&result.unwrap_err(), 6013, "Slippage exceeded.");

    // ensure state did not change
    let user_share_ata_account = svm.get_account(&user_share_ata).unwrap();
    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_after, 0);

    let reserve_account = svm.get_account(&reserve_pubkey).unwrap();
    let reserve_balance_after = TokenAccount::unpack(reserve_account.data()).unwrap().amount;
    assert_eq!(reserve_balance_after, 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, 0);
}

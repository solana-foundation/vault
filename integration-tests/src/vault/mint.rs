use anchor_spl::{
    token::{self},
    token_2022,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;
use vault_client::{sdk::program_id, FeeType, Pubkey, Vault};

use crate::helper_functions::{
    assert_error_code, create_ata, create_mint, create_mint_with_transfer_fee, get_fee,
    get_mint_supply, get_token_account_amount, get_vault_asset_balance, helper_mint_to, mint,
    recv_amount_from_params, set_up_vault,
};

#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token::ID,
    token::ID;
    "Mint (SPL Token asset, SPL Token share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token_2022::ID,
    token::ID;
    "Mint (Token 2022 asset, SPL Token share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token::ID,
    token_2022::ID;
    "Mint (SPL Token asset, Token 2022 share)"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),
    token_2022::ID,
    token_2022::ID;
    "Mint (Token 2022 asset, Token 2022 share)"
)]
fn test_mint_vault(fee_type: Option<FeeType>, asset_program: Pubkey, share_program: Pubkey) {
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
        create_mint(&mut svm, &mint_authority, &asset_mint, &token::ID);
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
        create_mint(&mut svm, &mint_authority, &share_mint, &token::ID);
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
        fee_type.clone(),
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

    let mint_amount = 500_000;
    let result = mint(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        mint_amount,
        u64::MAX, // no slippage protection
        asset_program,
        share_program,
    );

    assert!(result.is_ok(), "Unexpected result for test case");

    // Calculate expected values accounting for asset transfer fees
    let vault = svm.get_account(&vault_pubkey).unwrap();
    let vault_cfg = Vault::from_bytes(vault.data()).unwrap();

    let asset_amount: u64 = (mint_amount as u128)
        .checked_mul(vault_cfg.initial_price as u128)
        .unwrap()
        .try_into()
        .unwrap();

    // Mint uses get_deposit_fee_when_minting: gross = net * 10000 / (10000 - bps), fee = gross -
    // net
    let vault_fee = match fee_type {
        Some(FeeType::Percentage { bps }) => {
            let gross = (asset_amount as u128) * 10_000 / (10_000 - bps as u128);
            (gross - asset_amount as u128) as u64
        }
        Some(FeeType::FixedAmount { amount }) => amount,
        None => 0,
    };

    // The program adds transfer fee estimate on top of assets for the reserve transfer
    let transfer_fee_on_assets = asset_amount
        .checked_sub(recv_amount_from_params(
            asset_amount,
            asset_transfer_fee_bps,
            asset_transfer_fee_max,
        ))
        .unwrap();
    let gross_to_reserve = asset_amount
        .checked_add(transfer_fee_on_assets)
        .expect("overflow");

    // User pays: gross_to_reserve (to reserve) + vault_fee (to fee_recipient)
    let total_user_paid = gross_to_reserve.checked_add(vault_fee).expect("overflow");

    // Recipients receive amounts after Token 2022 transfer fee withholding
    let fee_received =
        recv_amount_from_params(vault_fee, asset_transfer_fee_bps, asset_transfer_fee_max);
    let reserve_received = recv_amount_from_params(
        gross_to_reserve,
        asset_transfer_fee_bps,
        asset_transfer_fee_max,
    );

    // Assert post-mint state
    let fee_recipient_balance_after =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(fee_recipient_balance_after, fee_received);

    let user_asset_balance_after =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(total_user_paid)
            .expect("overflow")
    );

    let user_share_balance_after =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(user_share_balance_after, mint_amount);

    let reserve_balance_after =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_balance_after, reserve_received);

    let share_supply = get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(share_supply, mint_amount);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, reserve_received);
}

#[test]
fn test_mint_vault_slippage_protection_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();

    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    create_mint(&mut svm, &mint_authority, &asset_mint, &token::ID);
    create_mint(&mut svm, &mint_authority, &share_mint, &token::ID);

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

    // Bootstrap state: supply == 0 and total_assets == 0, so
    // assets_required = shares * initial_price
    let mint_amount = 123_456;

    let vault_acc = svm.get_account(&vault_pubkey).unwrap();
    let vault_cfg = Vault::from_bytes(vault_acc.data()).unwrap();

    let assets_required: u64 = (mint_amount as u128)
        .checked_mul(vault_cfg.initial_price as u128)
        .unwrap()
        .try_into()
        .unwrap();

    let max_assets = assets_required - 1;

    // Snapshot state for "no side effects" check
    let user_share_before = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let vault_asset_balance_before = get_vault_asset_balance(&svm, &vault_pubkey);

    let result = mint(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        mint_amount,
        max_assets,
        token::ID,
        token::ID,
    );

    assert_error_code(&result.unwrap_err(), 6013, "Slippage exceeded.");

    // Ensure state did not change
    let user_share_after = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_after = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let vault_asset_balance_after = get_vault_asset_balance(&svm, &vault_pubkey);

    assert_eq!(user_share_after, user_share_before);
    assert_eq!(reserve_after, reserve_before);
    assert_eq!(vault_asset_balance_after, vault_asset_balance_before);
}

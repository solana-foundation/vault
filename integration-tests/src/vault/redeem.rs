use anchor_spl::{
    token,
    token_2022::{self, spl_token_2022},
};
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::InstructionError, signature::Keypair, signer::Signer,
    transaction::TransactionError,
};
use vault_client::{sdk::program_id, FeeType, Pubkey};

use crate::vault::helper_functions::{
    assert_error_code, create_ata, create_mint, create_mint_with_transfer_fee, deposit, get_fee,
    get_mint_supply, get_token_account_amount, get_vault_asset_balance, helper_mint_to,
    recv_amount_from_params, redeem, set_up_vault,
};
use test_case::test_case;

/// Mirrors get_withdraw_fee_when_redeeming formula:
/// fee = ceil(gross * bps / (MAX_BPS + bps))
fn redeem_fee_from_gross(fee: FeeType, gross: u64) -> u64 {
    match fee {
        FeeType::Percentage { bps } => {
            if bps == 0 {
                return 0;
            }
            let denominator = 10_000u128 + bps as u128;
            let numerator = gross as u128 * bps as u128;
            u64::try_from(numerator.div_ceil(denominator)).expect("overflow")
        }
        FeeType::FixedAmount { amount } => amount,
        FeeType::NoFee => 0,
    }
}

/// Mirrors get_assets_from_shares formula:
fn assets_from_shares_formula(
    total_assets: u64,
    shares_supply: u64,
    share_amount: u64,
    round_up: bool,
) -> u64 {
    assert!(
        shares_supply > 0,
        "shares_supply must be > 0 (InvalidState on-chain)"
    );

    let numerator = u128::from(share_amount)
        .checked_mul(u128::from(total_assets))
        .expect("overflow");

    let denominator = u128::from(shares_supply + 1);

    let q = if round_up {
        numerator.div_ceil(denominator)
    } else {
        numerator.checked_div(denominator).expect("overflow")
    };

    u64::try_from(q).expect("result doesn't fit u64")
}

#[test_case(
    FeeType::Percentage { bps: 100 },  // 1% deposit fee
    FeeType::Percentage { bps: 50 },
    token::ID; // 0.5% withdraw/redeem fee
    "Redeem successfully (percentage fees) token keg"
)]
#[test_case(
    FeeType::NoFee,
    FeeType::NoFee,
    token::ID;
    "Redeem successfully (no fees) token keg"
)]
#[test_case(
    FeeType::Percentage { bps: 100 },  // 1% deposit fee
    FeeType::Percentage { bps: 50 },
    token_2022::ID; // 0.5% withdraw/redeem fee
    "Redeem successfully (percentage fees) token 2022 and transfer fee"
)]
#[test_case(
    FeeType::NoFee,
    FeeType::NoFee,
    token_2022::ID;
    "Redeem successfully (no fees) token 2022 and transfer fee"
)]
fn test_redeem_vault(deposit_fee: FeeType, withdraw_fee: FeeType, token_program: Pubkey) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);

    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    let mut transfer_fee_bps = 0;
    let mut transfer_fee_max_fee = 0;

    if token_program == token::ID {
        create_mint(&mut svm, &mint_authority, &asset_mint);
        create_mint(&mut svm, &mint_authority, &share_mint);
    } else {
        transfer_fee_bps = 10;
        transfer_fee_max_fee = 1000;
        create_mint_with_transfer_fee(
            &mut svm,
            &mint_authority,
            &asset_mint,
            transfer_fee_bps,
            transfer_fee_max_fee,
        );
        create_mint_with_transfer_fee(
            &mut svm,
            &mint_authority,
            &share_mint,
            transfer_fee_bps,
            transfer_fee_max_fee,
        );
    }

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token_program,
        token_program,
        &deposit_fee,
        &withdraw_fee,
    );

    let fee_recipient_ata = create_ata(
        &mut svm,
        &fee_recipient,
        &asset_mint.pubkey(),
        &token_program,
    );
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token_program);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token_program);

    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token_program,
    );

    // -------------------- deposit --------------------
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
        token_program,
        token_program,
    );
    assert!(result.is_ok(), "deposit failed unexpectedly");

    let deposit_fee_amount = get_fee(deposit_fee.clone(), deposit_amount);
    let deposit_net = deposit_amount
        .checked_sub(deposit_fee_amount)
        .expect("overflow");

    let deposit_fee_received =
        recv_amount_from_params(deposit_fee_amount, transfer_fee_bps, transfer_fee_max_fee);
    let deposit_net_received =
        recv_amount_from_params(deposit_net, transfer_fee_bps, transfer_fee_max_fee);

    // -------------------- state after deposit --------------------
    let fee_recipient_after_deposit =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(fee_recipient_after_deposit, deposit_fee_received);

    let reserve_after_deposit =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_after_deposit, deposit_net_received);

    let user_shares_after_deposit =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(user_shares_after_deposit, deposit_net_received);

    let share_supply_after_deposit =
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(share_supply_after_deposit, deposit_net_received);

    let vault_asset_balance_after_deposit = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(
        vault_asset_balance_after_deposit, deposit_net_received,
        "Vault internal balance should be equal to deposit_net_received"
    );

    // -------------------- redeem --------------------
    // `shares` is the input. We compute total_assets_out from shares (round down),
    // then fee is applied on total_assets_out and user receives net assets.
    let redeem_shares: u64 = 100_000;

    let total_assets_out = assets_from_shares_formula(
        vault_asset_balance_after_deposit, // total_assets
        share_supply_after_deposit,        // shares_supply
        redeem_shares,                     // share_amount
        false,                             // Rounding::Down
    );

    let redeem_fee_amount = redeem_fee_from_gross(withdraw_fee.clone(), total_assets_out);

    let user_assets_out = total_assets_out
        .checked_sub(redeem_fee_amount)
        .expect("overflow");

    assert!(user_assets_out > 0, "test expects non-zero net assets out");

    let redeem_fee_received =
        recv_amount_from_params(redeem_fee_amount, transfer_fee_bps, transfer_fee_max_fee);
    let user_assets_received =
        recv_amount_from_params(user_assets_out, transfer_fee_bps, transfer_fee_max_fee);

    let user_assets_before_redeem =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());

    let result = redeem(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        redeem_shares,
        0, // no slippage protection
        token_program,
        token_program,
    );
    assert!(result.is_ok(), "redeem failed unexpectedly");

    // -------------------- assert post-redeem --------------------
    // fee recipient total = deposit_fee + redeem_fee
    let fee_recipient_after_redeem =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(
        fee_recipient_after_redeem,
        deposit_fee_received + redeem_fee_received
    );

    // user assets increase by net amount received
    let user_assets_after_redeem =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(
        user_assets_after_redeem,
        user_assets_before_redeem + user_assets_received
    );

    // reserve decreases by total assets withdrawn (fee + user net)
    let reserve_after_redeem = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(
        reserve_after_redeem,
        reserve_after_deposit - total_assets_out
    );

    // user shares decrease by shares burned
    let user_shares_after_redeem =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(
        user_shares_after_redeem,
        user_shares_after_deposit - redeem_shares
    );

    // share supply decreases by shares burned
    let share_supply_after_redeem =
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(
        share_supply_after_redeem,
        share_supply_after_deposit - redeem_shares
    );

    // vault internal assets decrease by total assets withdrawn
    let vault_asset_balance_after_redeem = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(
        vault_asset_balance_after_redeem,
        deposit_net_received - total_assets_out,
        "Vault internal balance should be deposit_net_received - total_assets_out"
    );

    // ---------- redeem fails (not enough shares) ------------
    let failing_shares = user_shares_after_redeem.checked_add(1).unwrap();

    let result = redeem(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        failing_shares,
        0, // no slippage protection
        token_program,
        token_program,
    );

    let Err(error) = result else {
        panic!("redeem should have failed");
    };

    // SPL token burn should fail with InsufficientFunds when user doesn't have enough shares
    let error_code = match error.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => code,
        other => panic!("unexpected tx error (not Custom): {:?}", other),
    };
    if token_program == token::ID {
        assert_eq!(
            error_code,
            spl_token::error::TokenError::InsufficientFunds as u32
        );
    } else {
        assert_eq!(
            error_code,
            spl_token_2022::error::TokenError::InsufficientFunds as u32
        );
    }
}

#[test]
fn test_redeem_slippage_protection() {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);

    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();

    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);

    // deposit fee 1%, redeem fee 0.5% (so redeem output is predictable)
    let deposit_fee = FeeType::Percentage { bps: 100 };
    let redeem_fee = FeeType::Percentage { bps: 50 };

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token::ID,
        token::ID,
        &deposit_fee,
        &redeem_fee,
    );

    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    // fund user
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        100_000_000,
        &token::ID,
    );

    // deposit first (no slippage)
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
        token::ID,
        token::ID,
    );
    assert!(result.is_ok(), "deposit failed unexpectedly");

    // expected shares minted = deposit_amount - deposit_fee
    let deposit_fee_amt = get_fee(deposit_fee.clone(), deposit_amount);
    let minted_shares = deposit_amount.checked_sub(deposit_fee_amt).unwrap();

    // redeem with forced slippage failure
    let redeem_shares: u64 = 100_000;

    // matches on-chain: total_assets_out = floor(shares * total_assets / (supply + 1))
    let total_assets_out = assets_from_shares_formula(
        minted_shares, // total_assets tracked by vault after deposit (no transfer fee here)
        minted_shares, // share supply after deposit
        redeem_shares,
        false, // Rounding::Down
    );

    let redeem_fee_amt = redeem_fee_from_gross(redeem_fee.clone(), total_assets_out);
    let user_assets_out = total_assets_out.checked_sub(redeem_fee_amt).unwrap();
    assert!(user_assets_out > 0);

    // force slippage failure: ask for more assets than user can receive
    let min_assets = user_assets_out + 1;

    // snapshot balances before failing redeem
    let user_assets_before = get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    let user_shares_before = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let fee_recipient_before =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    let vault_asset_balance_before = get_vault_asset_balance(&svm, &vault_pubkey);

    let result = redeem(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        redeem_shares,
        min_assets,
        token::ID,
        token::ID,
    );

    assert_error_code(
        &result.unwrap_err(),
        6013, // SlippageExceeded
        "Slippage exceeded.",
    );

    // ensure state did not change
    let user_assets_after = get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    let user_shares_after = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_after = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let fee_recipient_after =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    let vault_asset_balance_after = get_vault_asset_balance(&svm, &vault_pubkey);

    assert_eq!(user_assets_after, user_assets_before);
    assert_eq!(user_shares_after, user_shares_before);
    assert_eq!(reserve_after, reserve_before);
    assert_eq!(fee_recipient_after, fee_recipient_before);
    assert_eq!(vault_asset_balance_after, vault_asset_balance_before);
}

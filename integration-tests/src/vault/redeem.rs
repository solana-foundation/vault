use anchor_spl::token::{self, spl_token::error::TokenError};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, instruction::InstructionError, program_pack::Pack,
    signature::Keypair, signer::Signer, transaction::TransactionError,
};
use spl_token::state::{Account as TokenAccount, Mint as TokenMint};
use vault_client::{sdk::program_id, FeeType, VaultConfig};

use crate::vault::helper_functions::{
    create_ata, create_mint, deposit, get_fee, helper_mint_to, redeem, set_up_vault,
};
use test_case::test_case;

// Mirrors get_assets_from_shares formula:
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

    let denominator = u128::from(shares_supply);

    let q = if round_up {
        numerator.div_ceil(denominator)
    } else {
        numerator.checked_div(denominator).expect("overflow")
    };

    u64::try_from(q).expect("result doesn't fit u64")
}

#[test_case(
    FeeType::Percentage { bps: 100 },  // 1% deposit fee
    FeeType::Percentage { bps: 50 };   // 0.5% withdraw/redeem fee
    "Redeem successfully (percentage fees)"
)]
#[test_case(
    FeeType::NoFee,
    FeeType::NoFee;
    "Redeem successfully (no fees)"
)]
fn test_redeem_vault(deposit_fee: FeeType, withdraw_fee: FeeType) {
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

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token::ID,
        token::ID,
        &deposit_fee,
        &withdraw_fee,
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
        token::ID,
        token::ID,
    );
    assert!(result.is_ok(), "deposit failed unexpectedly");

    let deposit_fee_amount = get_fee(deposit_fee.clone(), deposit_amount);
    let deposit_net = deposit_amount
        .checked_sub(deposit_fee_amount)
        .expect("overflow");

    // -------------------- state after deposit --------------------
    let fee_recipient_after_deposit =
        TokenAccount::unpack(svm.get_account(&fee_recipient_ata).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(fee_recipient_after_deposit, deposit_fee_amount);

    let reserve_after_deposit =
        TokenAccount::unpack(svm.get_account(&reserve_pubkey).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(reserve_after_deposit, deposit_net);

    let user_shares_after_deposit =
        TokenAccount::unpack(svm.get_account(&user_share_ata).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(user_shares_after_deposit, deposit_net);

    let share_supply_after_deposit =
        TokenMint::unpack(svm.get_account(&share_mint.pubkey()).unwrap().data())
            .unwrap()
            .supply;
    assert_eq!(share_supply_after_deposit, deposit_net);

    let vault_after_deposit = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_cfg_after_deposit = VaultConfig::from_bytes(vault_after_deposit.data()).unwrap();
    assert_eq!(
        vault_cfg_after_deposit.total_asset_balance, deposit_net,
        "Vault internal balance should be equal to deposit_net"
    );

    // -------------------- redeem --------------------
    // `shares` is the input. We compute total_assets_out from shares (round down),
    // then fee is applied on total_assets_out and user receives net assets.
    let redeem_shares: u64 = 100_000;

    let total_assets_out = assets_from_shares_formula(
        vault_cfg_after_deposit.total_asset_balance, // total_assets
        share_supply_after_deposit,                  // shares_supply
        redeem_shares,                               // share_amount
        false,                                       // Rounding::Down
    );

    let redeem_fee_amount = get_fee(withdraw_fee.clone(), total_assets_out);

    let user_assets_out = total_assets_out
        .checked_sub(redeem_fee_amount)
        .expect("overflow");
    assert!(user_assets_out > 0, "test expects non-zero net assets out");

    let user_assets_before_redeem =
        TokenAccount::unpack(svm.get_account(&user_asset_ata).unwrap().data())
            .unwrap()
            .amount;

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
        token::ID,
        token::ID,
    );
    assert!(result.is_ok(), "redeem failed unexpectedly");

    // -------------------- assert post-redeem --------------------
    // fee recipient total = deposit_fee + redeem_fee
    let fee_recipient_after_redeem =
        TokenAccount::unpack(svm.get_account(&fee_recipient_ata).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(
        fee_recipient_after_redeem,
        deposit_fee_amount + redeem_fee_amount
    );

    // user assets increase by net amount received
    let user_assets_after_redeem =
        TokenAccount::unpack(svm.get_account(&user_asset_ata).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(
        user_assets_after_redeem,
        user_assets_before_redeem + user_assets_out
    );

    // reserve decreases by total assets withdrawn (fee + user net)
    let reserve_after_redeem =
        TokenAccount::unpack(svm.get_account(&reserve_pubkey).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(
        reserve_after_redeem,
        reserve_after_deposit - total_assets_out
    );

    // user shares decrease by shares burned
    let user_shares_after_redeem =
        TokenAccount::unpack(svm.get_account(&user_share_ata).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(
        user_shares_after_redeem,
        user_shares_after_deposit - redeem_shares
    );

    // share supply decreases by shares burned
    let share_supply_after_redeem =
        TokenMint::unpack(svm.get_account(&share_mint.pubkey()).unwrap().data())
            .unwrap()
            .supply;
    assert_eq!(
        share_supply_after_redeem,
        share_supply_after_deposit - redeem_shares
    );

    // vault internal assets decrease by total assets withdrawn
    let vault_after_redeem = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_cfg_after_redeem = VaultConfig::from_bytes(vault_after_redeem.data()).unwrap();
    assert_eq!(
        vault_cfg_after_redeem.total_asset_balance,
        deposit_net - total_assets_out,
        "Vault internal balance should be deposit_net - total_assets_out"
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
        token::ID,
        token::ID,
    );

    let Err(error) = result else {
        panic!("redeem should have failed");
    };

    // SPL token burn should fail with InsufficientFunds when user doesn't have enough shares
    let error_code = match error.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => code,
        other => panic!("unexpected tx error (not Custom): {:?}", other),
    };
    assert_eq!(error_code, TokenError::InsufficientFunds as u32);
}

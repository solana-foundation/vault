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
    recv_amount_from_params, set_up_vault, withdraw,
};
use test_case::test_case;

#[test_case(
    Some(FeeType::Percentage { bps: 100 }),  // 1% deposit fee
    Some(FeeType::Percentage { bps: 50 }),
    token::ID;   // 0.5% withdraw fee
    "Withdraw successfully (percentage fees) token keg"
)]
#[test_case(
    None,
    None,
    token::ID;
    "Withdraw successfully (no fees) token keg"
)]
#[test_case(
    Some(FeeType::Percentage { bps: 100 }),  // 1% deposit fee
    Some(FeeType::Percentage { bps: 50 }),
    token_2022::ID;
    "Withdraw successfully (percentage fees) token 2022 and transfer fee"
)]
#[test_case(
    None,
    None,
    token_2022::ID;
    "Withdraw successfully (no fees) token 2022 and transfer fee"
)]
fn test_withdraw_vault(
    deposit_fee: Option<FeeType>,
    withdraw_fee: Option<FeeType>,
    token_program: Pubkey,
) {
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
        deposit_fee.clone(),
        withdraw_fee.clone(),
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

    // -------------------- balances before deposit --------------------
    let fee_recipient_balance_before =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(fee_recipient_balance_before, 0);

    let user_asset_balance_before =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let user_share_balance_before =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(user_share_balance_before, 0);

    let reserve_balance_before =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    // newly created reserve token account should have 0 assets
    assert_eq!(reserve_balance_before, 0);

    let share_supply_before_deposit =
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(share_supply_before_deposit, 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, 0, "Vault internal balance should be 0");

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

    // After deposit:
    // - fee recipient got deposit_fee_amount
    // - reserve got deposit_net
    // - user assets decreased by deposit_amount
    // - user shares minted = deposit_net
    let fee_recipient_after_deposit =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(fee_recipient_after_deposit, deposit_fee_received);

    let reserve_after_deposit =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_after_deposit, deposit_net_received);

    let user_assets_after_deposit =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(
        user_assets_after_deposit,
        user_asset_amount - deposit_amount
    );

    let user_shares_after_deposit =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(user_shares_after_deposit, deposit_net_received);

    let share_supply_after_deposit =
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(share_supply_after_deposit, deposit_net_received);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_net_received);

    // -------------------- withdraw --------------------
    // `assets_out` is NET to user. Vault additionally pays `withdraw_fee(assets_out)` out of
    // reserve. shares burned should cover gross = assets_out + fee.

    // assets_out small enough so gross <= user_shares_after_deposit.
    let assets_out: u64 = 100_000;

    let withdraw_fee_amount = get_fee(withdraw_fee.clone(), assets_out);
    let gross_amount = assets_out
        .checked_add(withdraw_fee_amount)
        .expect("overflow");

    let withdraw_fee_received =
        recv_amount_from_params(withdraw_fee_amount, transfer_fee_bps, transfer_fee_max_fee);
    let user_assets_received =
        recv_amount_from_params(assets_out, transfer_fee_bps, transfer_fee_max_fee);

    let result = withdraw(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        assets_out, // NET to user,
        u64::MAX,   // no slippage protection
        token_program,
        token_program,
    );
    assert!(result.is_ok(), "withdraw failed unexpectedly");

    // -------------------- assert post-withdraw --------------------
    // fee recipient total = deposit_fee + withdraw_fee
    let fee_recipient_after_withdraw =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    assert_eq!(
        fee_recipient_after_withdraw,
        deposit_fee_received + withdraw_fee_received
    );

    // user assets: -deposit_amount + assets_out
    let user_assets_after_withdraw =
        get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    assert_eq!(
        user_assets_after_withdraw,
        (user_asset_amount - deposit_amount) + user_assets_received
    );

    // reserve: +deposit_net - gross
    let reserve_after_withdraw =
        get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    assert_eq!(reserve_after_withdraw, deposit_net_received - gross_amount);

    // user shares: +deposit_net - gross
    let user_shares_after_withdraw =
        get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    assert_eq!(
        user_shares_after_withdraw,
        deposit_net_received - gross_amount
    );

    // share supply: deposit_net - gross
    let share_supply_after_withdraw =
        get_mint_supply(&svm.get_account(&share_mint.pubkey()).unwrap());
    assert_eq!(
        share_supply_after_withdraw,
        deposit_net_received - gross_amount
    );

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(
        vault_asset_balance,
        deposit_net_received - gross_amount,
        "Vault internal balance should be deposit_net_received - gross_amount"
    );

    // ---------- withdraw fails (not enough shares) ------------
    let failing_assets_out = user_shares_after_withdraw.checked_add(1).unwrap();
    let result = withdraw(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        failing_assets_out,
        u64::MAX, // no slippage protection
        token_program,
        token_program,
    );

    let Err(error) = result else {
        panic!("withdraw should have failed");
    };

    // Extract the SPL token custom error code
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
fn test_withdraw_slippage_protection() {
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

    // keep it simple: token-keg, deposit fee 1%, withdraw fee 0.5%
    let deposit_fee = Some(FeeType::Percentage { bps: 100 });
    let withdraw_fee = Some(FeeType::Percentage { bps: 50 });

    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token::ID,
        token::ID,
        deposit_fee.clone(),
        withdraw_fee.clone(),
    );

    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    // fund user
    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID,
    );

    // --- deposit first (no slippage) ---
    let deposit_amount = 500_000;
    deposit(
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
    )
    .expect("deposit failed unexpectedly");

    // withdraw with forced slippage failure
    // assets_out is NET to user, fee is extra out of reserve (gross = assets_out + fee)
    let assets_out: u64 = 100_000;

    // force slippage failure: require burning fewer shares than needed (max_shares too small)
    let withdraw_fee_amt = get_fee(withdraw_fee.clone(), assets_out);
    let gross_needed = assets_out.checked_add(withdraw_fee_amt).unwrap();

    // withdraw burns shares for gross (with rounding). make max_shares definitely too small.
    let max_shares = gross_needed - 1;

    // snapshot state before failing withdraw
    let fee_recipient_before =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    let user_assets_before = get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    let user_shares_before = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_before = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let vault_asset_balance_before = get_vault_asset_balance(&svm, &vault_pubkey);

    let result = withdraw(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        assets_out,
        max_shares, // <-- slippage protection triggers
        token::ID,
        token::ID,
    );

    assert_error_code(
        &result.unwrap_err(),
        6013, // SlippageExceeded
        "Slippage exceeded.",
    );

    // ensure state did not change
    let fee_recipient_after =
        get_token_account_amount(&svm.get_account(&fee_recipient_ata).unwrap());
    let user_assets_after = get_token_account_amount(&svm.get_account(&user_asset_ata).unwrap());
    let user_shares_after = get_token_account_amount(&svm.get_account(&user_share_ata).unwrap());
    let reserve_after = get_token_account_amount(&svm.get_account(&reserve_pubkey).unwrap());
    let vault_asset_balance_after = get_vault_asset_balance(&svm, &vault_pubkey);

    assert_eq!(fee_recipient_after, fee_recipient_before);
    assert_eq!(user_assets_after, user_assets_before);
    assert_eq!(user_shares_after, user_shares_before);
    assert_eq!(reserve_after, reserve_before);
    assert_eq!(vault_asset_balance_after, vault_asset_balance_before);
}

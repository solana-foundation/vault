use anchor_spl::{
    token::{self},
    token_2022::{
        self,
        spl_token_2022::{self, extension::StateWithExtensions},
    },
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::Account as TokenAccount;
use spl_token_2022::state::Account as TokenAccount2022;
use vault_client::{sdk::program_id, FeeType};

use crate::vault::helper_functions::{
    assert_error_code, create_ata, create_mint, create_mint_with_transfer_fee, deposit, get_fee,
    get_vault_asset_balance, helper_mint_to, set_up_vault,
};

#[test]
fn test_deposit_vault() {
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
        &FeeType::Percentage { bps: 100 },
        &FeeType::NoFee,
    );
    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_asset_amount = 100_000_000;

    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID,
    );
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    let mut fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Fee recipient ata account should exist");

    let fee_recipient_balance_before = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(fee_recipient_balance_before, 0);

    let mut reserve_account = svm
        .get_account(&reserve_pubkey)
        .expect("Reserve account should exist");

    let reserve_balance_before = TokenAccount::unpack(reserve_account.data()).unwrap().amount;
    assert_eq!(reserve_balance_before, 0);

    let mut user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("User asset ata account should exist");

    let user_asset_balance_before = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let mut user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("User share ata should exist");

    let user_share_balance_before = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_before, 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, 0);

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
        0, // no slippage protection set
        token::ID,
        token::ID,
    );
    match &result {
        Ok(_) => (),
        Err(e) => {
            println!("error: {}", e.meta.pretty_logs());
        }
    };

    assert_eq!(result.is_ok(), true, "Unexpected result for test case");

    fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Fee recipient ata account should exist");

    let fee_recipient_balance_after = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;
    let fee = get_fee(FeeType::Percentage { bps: 100 }, deposit_amount);
    let deposit_amount_minus_fee = deposit_amount.checked_sub(fee).expect("overflow");
    assert_eq!(fee_recipient_balance_after, fee);

    user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("user asset ata account should exist");

    let user_asset_balance_after = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;

    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(deposit_amount)
            .expect("overflow")
    );

    user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("User share ata account should exist");

    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;

    assert_eq!(user_share_balance_after, deposit_amount_minus_fee);

    reserve_account = svm
        .get_account(&reserve_pubkey)
        .expect("reserve account should exist");

    let reserve_balance_after = TokenAccount::unpack(reserve_account.data()).unwrap().amount;
    assert_eq!(reserve_balance_after, deposit_amount_minus_fee);
    let share_mint_account = svm
        .get_account(&share_mint.pubkey())
        .expect("share mint account should exist");

    let mint_account = spl_token::state::Mint::unpack(&share_mint_account.data).unwrap();
    assert_eq!(mint_account.supply, deposit_amount_minus_fee);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_amount_minus_fee);
}

#[test]
fn test_deposit_vault_with_transfer_fees() {
    let mut svm = LiteSVM::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();
    let transfer_fee: u16 = 10;
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    create_mint_with_transfer_fee(&mut svm, &mint_authority, &asset_mint, transfer_fee, 1000);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);
    let (_, user, _, mint_authority, fee_recipient, reserve_pubkey, vault_pubkey) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token_2022::ID,
        token::ID,
        &FeeType::Percentage { bps: 100 },
        &FeeType::NoFee,
    );
    let fee_recipient_ata = create_ata(
        &mut svm,
        &fee_recipient,
        &asset_mint.pubkey(),
        &token_2022::ID,
    );
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token_2022::ID);
    let user_asset_amount = 100_000_000;

    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token_2022::ID,
    );
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    let mut fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Fee recipient ata account should exist");

    let fee_recipient_balance_before =
        StateWithExtensions::<TokenAccount2022>::unpack(fee_recipient_ata_account.data())
            .unwrap()
            .base
            .amount;
    assert_eq!(fee_recipient_balance_before, 0);

    let mut reserve_account = svm
        .get_account(&reserve_pubkey)
        .expect("Reserve account should exist");

    let reserve_balance_before =
        StateWithExtensions::<TokenAccount2022>::unpack(reserve_account.data())
            .unwrap()
            .base
            .amount;
    assert_eq!(reserve_balance_before, 0);

    let mut user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("User asset ata account should exist");

    let user_asset_balance_before =
        StateWithExtensions::<TokenAccount2022>::unpack(user_asset_ata_account.data())
            .unwrap()
            .base
            .amount;
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let mut user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("User share ata account should exist");

    let user_share_balance_before = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_before, 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, 0);
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
        0, // no slippage protection set
        token_2022::ID,
        token::ID,
    );

    assert_eq!(result.is_ok(), true, "Unexpected result for test case");

    fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Fee recipient ata account should exist");

    let fee_recipient_balance_after =
        StateWithExtensions::<TokenAccount2022>::unpack(fee_recipient_ata_account.data())
            .unwrap()
            .base
            .amount;
    let fee = get_fee(FeeType::Percentage { bps: 100 }, deposit_amount);
    let transfer_fee_amount = fee
        .checked_mul(transfer_fee.into())
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    let fee_minus_transfer_fee_amount = fee.checked_sub(transfer_fee_amount).unwrap();
    assert_eq!(fee_recipient_balance_after, fee_minus_transfer_fee_amount);

    user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("User asset account should exist");

    let user_asset_balance_after =
        StateWithExtensions::<TokenAccount2022>::unpack(user_asset_ata_account.data())
            .unwrap()
            .base
            .amount;

    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(deposit_amount)
            .expect("overflow")
    );

    user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("User share ata account should exist");

    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;

    let transfer_fee_deposit_amount = deposit_amount
        .checked_sub(fee)
        .unwrap()
        .checked_mul(transfer_fee.into())
        .unwrap()
        .checked_div(10_000)
        .unwrap();

    let deposit_amount_minus_fee_minus_transfer_fee_deposit_amount = deposit_amount
        .checked_sub(fee)
        .expect("overflow")
        .checked_sub(transfer_fee_deposit_amount)
        .expect("overflow");

    assert_eq!(
        user_share_balance_after,
        deposit_amount_minus_fee_minus_transfer_fee_deposit_amount
    );

    reserve_account = svm
        .get_account(&reserve_pubkey)
        .expect("Reserve account should exist");

    let reserve_balance_after =
        StateWithExtensions::<TokenAccount2022>::unpack(reserve_account.data())
            .unwrap()
            .base
            .amount;
    assert_eq!(
        reserve_balance_after,
        deposit_amount_minus_fee_minus_transfer_fee_deposit_amount
    );
    let share_mint_account = svm
        .get_account(&share_mint.pubkey())
        .expect("Share mint account should exist");

    let mint_account = spl_token::state::Mint::unpack(&share_mint_account.data).unwrap();
    assert_eq!(
        mint_account.supply,
        deposit_amount_minus_fee_minus_transfer_fee_deposit_amount
    );
    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(
        vault_asset_balance,
        deposit_amount_minus_fee_minus_transfer_fee_deposit_amount
    );
}

#[test]
fn test_deposit_slippage_protection() {
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
        &FeeType::Percentage { bps: 100 },
        &FeeType::NoFee,
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
    let fee = get_fee(FeeType::Percentage { bps: 100 }, deposit_amount);
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

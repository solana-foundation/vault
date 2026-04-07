use anchor_spl::{
    token,
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
use vault_client::{sdk::program_id, FeeType, Vault};

use crate::vault::helper_functions::{
    assert_error_code, create_ata, create_mint, create_mint_with_transfer_fee,
    get_vault_asset_balance, helper_mint_to, mint, recv_amount_from_params, set_up_vault,
};

#[test]
fn test_mint_vault() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);
    let authority = Keypair::new();
    let user = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

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
        .expect("Vault account should exist");

    let fee_recipient_balance_before = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(fee_recipient_balance_before, 0);

    let user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("Vault account should exist");

    let user_asset_balance_before = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let mut user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("Vault account should exist");

    let user_share_balance_before = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_before, 0);
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
        token::ID,
        token::ID,
    );

    assert_eq!(result.is_ok(), true, "Unexpected result for test case");

    fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Vault account should exist");

    let fee_recipient_balance_after = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;

    let vault = svm.get_account(&vault_pubkey).unwrap();
    let vault_cfg = Vault::from_bytes(vault.data()).unwrap();

    let asset_amount: u64 = (mint_amount as u128)
        .checked_mul(vault_cfg.initial_price as u128)
        .unwrap()
        .try_into()
        .unwrap();

    let user_asset_balance_after =
        TokenAccount::unpack(svm.get_account(&user_asset_ata).unwrap().data())
            .unwrap()
            .amount;

    let fee = user_asset_amount
        .checked_sub(user_asset_balance_after).unwrap() // total the user lost
        .checked_sub(asset_amount).unwrap(); // minus what went to the vault

    assert_eq!(fee_recipient_balance_after, fee);

    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(asset_amount)
            .expect("overflow")
            .checked_sub(fee)
            .expect("overflow")
    );

    user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("Vault account should exist");

    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_after, mint_amount);
}

#[test]
fn test_mint_vault_with_transfer_fees() {
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
        Some(FeeType::Percentage { bps: 100 }),
        None,
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

    user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("User asset account should exist");

    let user_asset_balance_after =
        StateWithExtensions::<TokenAccount2022>::unpack(user_asset_ata_account.data())
            .unwrap()
            .base
            .amount;

    reserve_account = svm
        .get_account(&reserve_pubkey)
        .expect("Reserve account should exist");
    let reserve_balance_after =
        StateWithExtensions::<TokenAccount2022>::unpack(reserve_account.data())
            .unwrap()
            .base
            .amount;

    let transfer_fee_withheld = user_asset_amount
        .checked_sub(user_asset_balance_after)
        .unwrap()
        .checked_sub(
            reserve_balance_after
                .checked_add(fee_recipient_balance_after)
                .unwrap(),
        )
        .unwrap();

    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(
                reserve_balance_after
                    .checked_add(fee_recipient_balance_after)
                    .unwrap()
                    .checked_add(transfer_fee_withheld)
                    .unwrap()
            )
            .unwrap()
    );

    user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("User share ata account should exist");

    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;

    let deposit_amount_minus_fee_minus_transfer_fee_deposit_amount = reserve_balance_after;

    assert_eq!(user_share_balance_after, mint_amount);

    assert_eq!(
        reserve_balance_after,
        deposit_amount_minus_fee_minus_transfer_fee_deposit_amount
    );
    let share_mint_account = svm
        .get_account(&share_mint.pubkey())
        .expect("Share mint account should exist");

    let mint_account = spl_token::state::Mint::unpack(&share_mint_account.data).unwrap();
    assert_eq!(mint_account.supply, user_share_balance_after);

    let vault = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault.data()).unwrap();

    let assets: u64 = (mint_amount as u128)
        .checked_mul(vault_config.initial_price as u128)
        .unwrap()
        .try_into()
        .unwrap();

    // Token2022 withholds its fee on the gross amount (assets + our estimate),
    // so the reserve receives slightly less than `assets`
    let fee_estimate = (assets as u128 * transfer_fee as u128).div_ceil(10_000) as u64;
    let gross = assets + fee_estimate;
    let expected_assets = recv_amount_from_params(gross, transfer_fee, 1000);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, expected_assets);
}

#[test]
fn test_mint_vault_slippage_protection_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);

    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();

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
    let user_share_before = TokenAccount::unpack(svm.get_account(&user_share_ata).unwrap().data())
        .unwrap()
        .amount;
    let reserve_before = TokenAccount::unpack(svm.get_account(&reserve_pubkey).unwrap().data())
        .unwrap()
        .amount;
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
    let user_share_after = TokenAccount::unpack(svm.get_account(&user_share_ata).unwrap().data())
        .unwrap()
        .amount;
    let reserve_after = TokenAccount::unpack(svm.get_account(&reserve_pubkey).unwrap().data())
        .unwrap()
        .amount;
    let vault_asset_balance_after = get_vault_asset_balance(&svm, &vault_pubkey);

    assert_eq!(user_share_after, user_share_before);
    assert_eq!(reserve_after, reserve_before);
    assert_eq!(vault_asset_balance_after, vault_asset_balance_before);
}

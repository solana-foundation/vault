use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, FeeType, Pubkey, Vault, VaultExtension};

use crate::vault::helper_functions::{
    assert_error_code, create_mint, create_vault, init_deposit_fees, init_vault,
    init_withdrawal_fees, update_deposit_fees, update_vault, update_withdrawal_fees,
};

#[test]
fn test_initialize_and_update_fees() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[b"reserve", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );

    // create and update vault so its ready to be used
    create_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        0,
        100_000,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    )
    .expect("vault creation failed");

    update_vault(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        100_000_000,
        false,
        authority.pubkey(),
    )
    .expect("vault update failed");

    // assert vault has no extensions yet
    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    assert!(vault_config.extensions.is_empty());

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    let withdrawal_fee = FeeType::Percentage { bps: 10 };

    // init deposit fee
    init_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &deposit_fee,
    )
    .expect("init deposit fee failed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    // deposit fee should be at index 0
    assert_eq!(
        vault_config.extensions[0],
        VaultExtension::DepositFee(deposit_fee)
    );

    // init withdrawal fee
    init_withdrawal_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &withdrawal_fee,
    )
    .expect("init withdrawal fee failed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    // withdrawal fee should be at index 1
    assert_eq!(
        vault_config.extensions[1],
        VaultExtension::WithdrawalFee(withdrawal_fee)
    );

    let new_deposit_fee = FeeType::FixedAmount { amount: 200 };
    let new_withdrawal_fee = FeeType::Percentage { bps: 20 };

    // update deposit fee
    update_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &new_deposit_fee,
    )
    .expect("update deposit fee failed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    // deposit fee should be at index 0
    assert_eq!(
        vault_config.extensions[0],
        VaultExtension::DepositFee(new_deposit_fee)
    );

    // update withdrawal fee
    update_withdrawal_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &new_withdrawal_fee,
    )
    .expect("update withdrawal fee failed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    // withdrawal fee should be at index 1
    assert_eq!(
        vault_config.extensions[1],
        VaultExtension::WithdrawalFee(new_withdrawal_fee)
    );
}

#[test]
fn test_initialize_fees_after_vault_initialization_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[b"reserve", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );

    // create and update vault so its ready to be used
    create_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        0,
        100_000,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    )
    .expect("vault creation failed");

    init_vault(&mut svm, &authority, &share_mint.pubkey(), &vault_pubkey)
        .expect("init vault failed");

    let deposit_fee = FeeType::FixedAmount { amount: 100 };

    // init deposit fee
    let result = init_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &deposit_fee,
    );

    assert_error_code(&result.unwrap_err(), 6014, "Vault is already initialized.");
}

#[test]
fn test_initialize_same_fee_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[b"reserve", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );

    create_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        0,
        100_000,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    )
    .expect("vault creation failed");

    let deposit_fee = FeeType::FixedAmount { amount: 100 };

    init_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &deposit_fee,
    )
    .expect("first init fee failed");

    svm.expire_blockhash();

    // init deposit fee
    let result = init_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &deposit_fee,
    );

    assert_error_code(
        &result.unwrap_err(),
        6015,
        "The extension is already initialized.",
    );
}

#[test]
fn test_update_before_init_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[b"reserve", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );

    // create and update vault so its ready to be used
    create_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        0,
        100_000,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    )
    .expect("vault creation failed");

    let deposit_fee = FeeType::FixedAmount { amount: 100 };

    // update deposit fee
    let result = update_deposit_fees(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        &deposit_fee,
    );

    assert_error_code(
        &result.unwrap_err(),
        6017,
        "The extension is not initialized",
    );
}

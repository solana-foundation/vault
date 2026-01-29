use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::Account as TokenAccount;
use vault_client::{sdk::program_id, FeeType, Pubkey, VaultConfig};

use crate::vault::{
    constants::{RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{close_vault, create_ata, create_mint, create_vault, helper_mint_to},
};

#[test]
fn test_close_vault() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[
            RESERVE_CONFIG_SEED,
            asset_mint.pubkey().as_ref(),
            share_mint.pubkey().as_ref(),
        ],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[
            VAULT_CONFIG_SEED,
            asset_mint.pubkey().as_ref(),
            share_mint.pubkey().as_ref(),
        ],
        &vault_client::sdk::program_id(),
    );
    let _ = create_vault(
        &mut svm,
        &authority,
        &payer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        FeeType::NoFee,
        FeeType::NoFee,
        0,
        100_000,
    );

    // Verify vault was created
    let mut vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    assert!(!vault_account.data.is_empty(), "Vault should have data");

    let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();
    assert_eq!(vault_config.authority, authority.pubkey());
    assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
    assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
    assert_eq!(vault_config.deposit_fees, FeeType::NoFee);
    assert_eq!(vault_config.withdraw_fees, FeeType::NoFee);
    assert_eq!(vault_config.initial_price, 100_000);
    assert_eq!(vault_config.paused, true);
    assert_eq!(vault_config.total_asset_balance, 0);
    assert_eq!(vault_config.vault_asset_cap, 0);
    assert_eq!(vault_config.vault_token_account, reserve_pubkey);

    let destination_ata = create_ata(&mut svm, &payer, &asset_mint.pubkey());
    // mint to
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &reserve_pubkey,
        &mint_authority,
        100_000_000,
    );

    let mut destination_ata_account = svm
        .get_account(&destination_ata)
        .expect("Vault account should exist");

    let destination_balance_before = TokenAccount::unpack(destination_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(destination_balance_before, 0);
    let mut reserve_ata_account = svm
        .get_account(&reserve_pubkey)
        .expect("Vault account should exist");
    let reserve_balance_before = TokenAccount::unpack(reserve_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(reserve_balance_before, 100_000_000);
    let _ = close_vault(
        &mut svm,
        &authority,
        &payer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        destination_ata,
    );

    vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    assert!(vault_account.data.is_empty(), "Vault should not have data");
    destination_ata_account = svm
        .get_account(&destination_ata)
        .expect("Vault account should exist");

    let destination_balance_after = TokenAccount::unpack(destination_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(destination_balance_after, 100_000_000);
    reserve_ata_account = svm
        .get_account(&reserve_pubkey)
        .expect("Vault account should exist");
    assert!(
        reserve_ata_account.data.is_empty(),
        "Reserve ATA should not have data"
    );
}

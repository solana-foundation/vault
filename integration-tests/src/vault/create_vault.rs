use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, Pubkey, VaultConfig};

use crate::vault::{
    constants::{RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{create_mint, create_vault},
};

#[test]
fn test_create_vault() {
    let initial_price = 100_000_000;
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);
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
        &[RESERVE_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, share_mint.pubkey().as_ref()],
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
        initial_price,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    )
    .expect("vault creation should succeed");

    // Verify vault was created
    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    assert!(!vault_account.data.is_empty(), "Vault should have data");

    let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();
    assert_eq!(vault_config.authority, authority.pubkey());
    assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
    assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
    assert!(vault_config.extensions.is_empty());
    assert_eq!(vault_config.initial_price, initial_price);
    assert_eq!(vault_config.paused, true);
    assert_eq!(vault_config.vault_asset_cap, 0);
    assert_eq!(vault_config.vault_token_account, reserve_pubkey);
}

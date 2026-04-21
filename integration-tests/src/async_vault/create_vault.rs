use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::async_vault::{
    constants::{PENDING_VAULT_SEED, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{
        async_vault_program_id, create_async_vault, create_mint, AsyncVaultAccount,
    },
};

#[test_case(100_000_000, true, true ; "both async inflows and outflows")]
#[test_case(100_000_000, true, false ; "async inflows only")]
#[test_case(100_000_000, false, true ; "async outflows only")]
#[test_case(100_000_000, false, false ; "no async flows")]
#[test_case(1, true, true ; "minimum price")]
#[test_case(u64::MAX, true, true ; "maximum price")]
fn test_create_vault(initial_price: u64, async_inflows: bool, async_outflows: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(async_vault_program_id(), program_bytes)
        .unwrap();

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
        &[RESERVE_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );
    let (pending_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_VAULT_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &async_vault_program_id(),
    );

    create_async_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
        initial_price,
        async_inflows,
        async_outflows,
        token::ID,
        token::ID,
    )
    .expect("async vault creation should succeed");

    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    assert!(!vault_account.data.is_empty(), "Vault should have data");

    let vault_config = AsyncVaultAccount::from_bytes(vault_account.data()).unwrap();
    assert_eq!(vault_config.authority, authority.pubkey());
    assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
    assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
    assert_eq!(vault_config.vault_token_account, reserve_pubkey);
    assert_eq!(vault_config.pending_vault, pending_vault_pubkey);
    assert_eq!(vault_config.initial_price, initial_price);
    assert!(vault_config.paused);
    assert!(!vault_config.initialized);
    assert_eq!(vault_config.nav, 0);
    assert_eq!(vault_config.nav_version, 0);
    assert_eq!(vault_config.async_inflows, async_inflows);
    assert_eq!(vault_config.async_outflows, async_outflows);
    assert_eq!(vault_config.pending_async_requests, 0);
    assert_eq!(vault_config.total_asset_balance, 0);
}

use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, Pubkey, Vault};

use crate::vault::helper_functions::{create_mint, create_vault, update_vault};
use test_case::test_case;

#[test_case(true, false, 100_000,Keypair::new().pubkey(); "unpause and vault cap")]
fn test_update_vault(
    should_succeed: bool,
    updated_paused_status: bool,
    updated_vault_asset_cap: u64,
    updated_authority: Pubkey,
) {
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
    let _ = create_vault(
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
    );

    let update_result = update_vault(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        updated_vault_asset_cap,
        updated_paused_status,
        updated_authority,
    );

    assert_eq!(
        update_result.is_ok(),
        should_succeed,
        "Unexpected result for test case"
    );

    if should_succeed {
        // Verify vault was updated
        let vault_account = svm
            .get_account(&vault_pubkey)
            .expect("Vault account should exist");
        assert!(!vault_account.data.is_empty(), "Vault should have data");

        let vault_config = Vault::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_config.authority, updated_authority);
        assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
        assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
        assert_eq!(vault_config.initial_price, 100_000);
        assert_eq!(vault_config.paused, updated_paused_status);
        assert_eq!(vault_config.initialized, false);
        assert_eq!(vault_config.vault_asset_cap, updated_vault_asset_cap);
        assert_eq!(vault_config.vault_token_account, reserve_pubkey);
    }
}

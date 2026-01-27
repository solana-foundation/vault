use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, FeeType, Pubkey, VaultConfig};

use crate::vault::{
    constants::{RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{assert_error_code, create_mint, create_vault},
};
use test_case::test_case;

#[test_case(FeeType::NoFee, FeeType::NoFee, 100_000_000,true; "No Fees")]
#[test_case(FeeType::Percentage { bps: 10_001 }, FeeType::NoFee,100_000_000, false; "Deposit fee limit exceeded")]
#[test_case(FeeType::Percentage { bps: 9000 }, FeeType::Percentage { bps: 10_001 },100_000_000,false; "Withdraw fee limit exceeded")]
fn test_create_vault(
    deposit_fee: FeeType,
    withdraw_fee: FeeType,
    initial_price: u64,
    should_succeed: bool,
) {
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
    let result = create_vault(
        &mut svm,
        &authority,
        &payer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        deposit_fee.clone(),
        withdraw_fee.clone(),
        0,
        initial_price,
    );
    assert_eq!(
        result.is_ok(),
        should_succeed,
        "Unexpected result for test case"
    );
    if should_succeed {
        // Verify vault was created
        let vault_account = svm
            .get_account(&vault_pubkey)
            .expect("Vault account should exist");
        assert!(!vault_account.data.is_empty(), "Vault should have data");

        let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_config.authority, authority.pubkey());
        assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
        assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
        assert_eq!(vault_config.deposit_fees, deposit_fee);
        assert_eq!(vault_config.withdraw_fees, withdraw_fee);
        assert_eq!(vault_config.initial_price, initial_price);
        assert_eq!(vault_config.paused, true);
        assert_eq!(vault_config.total_asset_balance, 0);
        assert_eq!(vault_config.vault_asset_cap, 0);
        assert_eq!(vault_config.vault_token_account, reserve_pubkey);
    } else {
        let failed_tx = result.unwrap_err();
        assert_error_code(
            &failed_tx,
            6000,
            "The provided fee must not exceed 100% (10,000 bps).",
        )
    }
}

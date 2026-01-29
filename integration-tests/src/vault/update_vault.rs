use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, FeeType, Pubkey, VaultConfig};

use crate::vault::helper_functions::{assert_error_code, create_mint, create_vault, update_vault};
use test_case::test_case;

#[test_case(FeeType::NoFee, FeeType::NoFee, true,  FeeType::Percentage { bps: 10_000 },FeeType::Percentage { bps: 10_000 }, false, 100_000,Keypair::new().pubkey(); "No Fees, Change fee percentage, unpause and vault cap")]
#[test_case(FeeType::NoFee, FeeType::NoFee, true,  FeeType::FixedAmount { amount: 10_000 },FeeType::Percentage { bps: 10_000 }, false, 100_000,Keypair::new().pubkey(); "No Fees, Change fee fixed amount, unpause and vault cap")]
#[test_case(FeeType::NoFee, FeeType::NoFee, false,  FeeType::Percentage { bps: 10_001 },FeeType::Percentage { bps: 10_000 }, false, 100_000,Keypair::new().pubkey(); "No Fees, Change fee, Error from exceeded fee")]
fn test_update_vault(
    deposit_fee: FeeType,
    withdraw_fee: FeeType,
    should_succeed: bool,
    updated_deposit_fee: FeeType,
    updated_withdraw_fee: FeeType,
    updated_paused_status: bool,
    updated_vault_asset_cap: u64,
    updated_authority: Pubkey,
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
            b"reserve",
            asset_mint.pubkey().as_ref(),
            share_mint.pubkey().as_ref(),
        ],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[
            b"vault",
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
        deposit_fee.clone(),
        withdraw_fee.clone(),
        0,
        100_000,
    );

    let update_result = update_vault(
        &mut svm,
        &authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        updated_deposit_fee.clone(),
        updated_withdraw_fee.clone(),
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

        let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_config.authority, updated_authority);
        assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
        assert_eq!(vault_config.share_mint_address, share_mint.pubkey());
        assert_eq!(vault_config.deposit_fees, updated_deposit_fee);
        assert_eq!(vault_config.withdraw_fees, updated_withdraw_fee);
        assert_eq!(vault_config.initial_price, 100_000);
        assert_eq!(vault_config.paused, updated_paused_status);
        assert_eq!(vault_config.total_asset_balance, 0);
        assert_eq!(vault_config.vault_asset_cap, updated_vault_asset_cap);
        assert_eq!(vault_config.vault_token_account, reserve_pubkey);
    } else {
        let failed_tx = update_result.unwrap_err();
        assert_error_code(
            &failed_tx,
            6000,
            "The provided fee must not exceed 100% (10,000 bps).",
        )
    }
}

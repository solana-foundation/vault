use litesvm::LiteSVM;
use solana_sdk::{signature::Keypair, signer::Signer};
use vault_client::{sdk::program_id, FeeType, Pubkey};

use crate::vault::{
    constants::{RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED},
    helper_functions::{
        assert_error_code, close_vault, create_ata, create_mint, create_vault, helper_mint_to,
    },
};
use test_case::test_case;

#[test_case(true,true;"Close vault successfully")]
#[test_case(false,true;"Close vault fails. Supply should be zero error")]
#[test_case(true,false;"Close vault fails. Asset reserve should be empty")]
fn test_close_vault(supply_is_zero: bool, reserve_is_empty: bool) {
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
    if !supply_is_zero {
        let share_account = create_ata(&mut svm, &payer, &share_mint.pubkey());
        helper_mint_to(
            &mut svm,
            &share_mint.pubkey(),
            &share_account,
            &mint_authority,
            100_000,
        );
    }

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

    if !reserve_is_empty {
        helper_mint_to(
            &mut svm,
            &asset_mint.pubkey(),
            &reserve_pubkey,
            &mint_authority,
            100_000,
        );
    }

    // Verify vault was created
    let mut vault_account = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    assert!(!vault_account.data.is_empty(), "Vault should have data");

    let result = close_vault(
        &mut svm,
        &authority,
        &payer,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
    );

    if supply_is_zero && reserve_is_empty {
        vault_account = svm
            .get_account(&vault_pubkey)
            .expect("Vault account should exist");
        assert!(vault_account.data.is_empty(), "Vault should not have data");

        let reserve_ata_account = svm
            .get_account(&reserve_pubkey)
            .expect("Reserve account should exist but be zeroed");
        assert!(
            reserve_ata_account.data.is_empty(),
            "Reserve ATA should not have data"
        );
    } else {
        let err_result = &result.unwrap_err();
        if !supply_is_zero {
            assert_error_code(err_result, 6002, "The provided mint supply should be zero.");
        }
        if !reserve_is_empty {
            assert_error_code(
                err_result,
                6003,
                "The provided vault reserve should be empty in order to close it.",
            );
        }
    }
}

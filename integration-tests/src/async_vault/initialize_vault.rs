use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, initialize_async_vault, setup_async_vault};

#[test_case(true, false ; "succeeds and preserves other fields")]
#[test_case(true, true ; "already initialized fails")]
#[test_case(false, false ; "unauthorized signer fails")]
fn test_initialize_vault(use_valid_authority: bool, pre_initialize: bool) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (authority, _, _, share_mint, _, _, vault_pubkey) = setup_async_vault(&mut svm);

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_before = Vault::from_bytes(vault_account.data()).unwrap();
    assert!(!vault_before.initialized);

    if pre_initialize {
        initialize_async_vault(&mut svm, &authority, share_mint.pubkey(), vault_pubkey)
            .expect("pre-initialize should succeed");
        svm.expire_blockhash();
    }

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let effective_authority = if use_valid_authority {
        &authority
    } else {
        &unauthorized
    };

    let result = initialize_async_vault(
        &mut svm,
        effective_authority,
        share_mint.pubkey(),
        vault_pubkey,
    );

    let should_succeed = use_valid_authority && !pre_initialize;

    if should_succeed {
        result.expect("initialize vault should succeed");

        let vault_account = svm.get_account(&vault_pubkey).unwrap();
        let vault_after = Vault::from_bytes(vault_account.data()).unwrap();

        assert!(vault_after.initialized);
        assert_eq!(vault_before.authority, vault_after.authority);
        assert_eq!(
            vault_before.asset_mint_address,
            vault_after.asset_mint_address
        );
        assert_eq!(
            vault_before.share_mint_address,
            vault_after.share_mint_address
        );
        assert_eq!(
            vault_before.vault_token_account,
            vault_after.vault_token_account
        );
        assert_eq!(vault_before.initial_price, vault_after.initial_price);
        assert_eq!(vault_before.async_inflows, vault_after.async_inflows);
        assert_eq!(vault_before.async_outflows, vault_after.async_outflows);
        assert_eq!(vault_before.nav, vault_after.nav);
        assert_eq!(vault_before.nav_version, vault_after.nav_version);
        assert_eq!(vault_before.pending_vault, vault_after.pending_vault);
        assert_eq!(
            vault_before.pending_async_requests,
            vault_after.pending_async_requests
        );
        assert_eq!(
            vault_before.total_asset_balance,
            vault_after.total_asset_balance
        );
    } else {
        let err_result = &result.unwrap_err();
        if pre_initialize {
            assert_error_code(err_result, 6004, "VaultAlreadyInitialized");
        }
        if !use_valid_authority {
            assert_error_code(err_result, 6001, "UnauthorizedSigner");
        }
    }
}

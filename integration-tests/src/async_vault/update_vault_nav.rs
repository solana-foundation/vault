use anchor_spl::token;
use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{set_up_async_vault, update_vault_nav};

#[test_case(200 ; "update nav succeeds")]
#[test_case(0 ; "update nav to zero succeeds")]
#[test_case(u128::MAX ; "update nav to max succeeds")]

fn test_update_vault_nav(updated_nav: u128) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
    ) = set_up_async_vault(
        &mut svm,
        token::ID,
        None,
        token::ID,
        1_000_000_000,
        100_000_000,
    );

    let vault_before = Vault::from_bytes(
        svm.get_account(&vault_pubkey)
            .expect("vault should exist")
            .data(),
    )
    .unwrap();
    let nav_version_before = vault_before.nav_version;

    let result = update_vault_nav(&mut svm, &authority, vault_pubkey, updated_nav);

    result.expect("update_vault_nav should succeed");

    let vault_account = svm.get_account(&vault_pubkey).expect("vault should exist");
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();

    assert_eq!(vault_config.nav, updated_nav);
    assert_eq!(vault_config.nav_version, nav_version_before + 1);
}

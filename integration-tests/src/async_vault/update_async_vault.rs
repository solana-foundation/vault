use anchor_spl::token;
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, UpdateVaultBuilder as UpdateVaultAsyncBuilder, Vault,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, set_up_async_vault};

#[test_case(Some(true), false; "pause vault")]
#[test_case(Some(false), false; "unpause vault")]
#[test_case(None, true; "update fee_recipient only")]
fn test_update_async_vault(paused: Option<bool>, update_fee_recipient: bool) {
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
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_before = Vault::from_bytes(vault_account.data()).unwrap();

    let new_fee_recipient = Keypair::new();

    let mut builder = UpdateVaultAsyncBuilder::new();
    builder
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey);
    if let Some(p) = paused {
        builder.paused(p);
    }
    if update_fee_recipient {
        builder.fee_recipient(new_fee_recipient.pubkey());
    }

    builder
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update vault should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_after = Vault::from_bytes(vault_account.data()).unwrap();

    if let Some(p) = paused {
        assert_eq!(vault_after.paused, p);
    } else {
        assert_eq!(vault_after.paused, vault_before.paused);
    }

    if update_fee_recipient {
        assert_eq!(vault_after.fee_recipient, new_fee_recipient.pubkey());
    } else {
        assert_eq!(vault_after.fee_recipient, vault_before.fee_recipient);
    }

    assert_eq!(vault_after.authority, vault_before.authority);
    assert_eq!(vault_after.asset_mint, vault_before.asset_mint);
    assert_eq!(vault_after.share_mint, vault_before.share_mint);
    assert_eq!(vault_after.nav, vault_before.nav);
    assert_eq!(vault_after.nav_version, vault_before.nav_version);
}

#[test]
fn test_update_async_vault_unauthorized_signer_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();
    let (
        _authority,
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
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let result = UpdateVaultAsyncBuilder::new()
        .authority(unauthorized.pubkey())
        .share_mint(share_mint.pubkey())
        .paused(true)
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &unauthorized.pubkey(), &[&unauthorized]);

    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

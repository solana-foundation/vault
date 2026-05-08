use anchor_spl::token;
use async_vault_client::{
    extensions::pausable_redemptions, lite::SendTransaction, sdk::program_id,
    CreateRedeemRequestBuilder, InitializePausableRedemptionsBuilder,
    InitializeVaultBuilder as InitializeAsyncVaultBuilder, RequestArgs,
    UpdatePausableRedemptionsBuilder, UpdateVaultNavBuilder,
};
use litesvm::LiteSVM;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, set_share_balance, set_up_async_vault};

const NAV: u128 = 1_000_000_000;
const SHARE_AMOUNT: u64 = 1_000_000_000;

fn setup(
    paused: Option<bool>,
) -> (
    LiteSVM,
    Keypair, // authority
    Keypair, // asset_mint
    Keypair, // share_mint
    Keypair, // user
    Pubkey,  // vault_pubkey
    Pubkey,  // user_share_account
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _payer,
        _mint_authority,
        asset_mint,
        share_mint,
        user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    if let Some(p) = paused {
        InitializePausableRedemptionsBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .paused(p)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize_pausable_redemptions should succeed");
    }

    InitializeAsyncVaultBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("initialize vault should succeed");

    UpdateVaultNavBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .updated_nav(NAV)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update nav should succeed");

    set_share_balance(
        &mut svm,
        &user_share_account,
        &share_mint.pubkey(),
        SHARE_AMOUNT,
    );

    (
        svm,
        authority,
        asset_mint,
        share_mint,
        user,
        vault_pubkey,
        user_share_account,
    )
}

fn create_redeem_request(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault_pubkey: Pubkey,
    user_share_account: Pubkey,
    amount: u64,
) -> litesvm::types::TransactionResult {
    let request_keypair = Keypair::new();

    CreateRedeemRequestBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .request(request_keypair.pubkey())
        .vault(vault_pubkey)
        .user_share_account(user_share_account)
        .share_token_program(spl_token::ID)
        .args(RequestArgs {
            amount,
            operator: None,
        })
        .instruction()
        .send_transaction(svm, &user.pubkey(), &[user, &request_keypair])
}

#[test]
fn test_initialize_pausable_redemptions_paused_false() {
    let (mut svm, _authority, asset_mint, share_mint, user, vault_pubkey, user_share_account) =
        setup(Some(false));

    let pausable_redempts = pausable_redemptions::get_state(
        svm.get_account(&vault_pubkey).expect("vault exists").data(),
    )
    .expect("PausableRedemptions should be initialized");
    assert!(!pausable_redempts.paused);

    create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        SHARE_AMOUNT,
    )
    .expect("redeem should succeed when paused=false");
}

#[test_case(true, false, 6004, "VaultAlreadyInitialized" ; "after_vault_init")]
#[test_case(false, true, 6005, "ExtensionAlreadyInitialized" ; "duplicate")]
fn test_initialize_pausable_redemptions_fails(
    init_vault_first: bool,
    init_extension_first: bool,
    expected_error: u32,
    expected_name: &str,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        _user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    if init_vault_first {
        InitializeAsyncVaultBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize vault should succeed");
    }

    if init_extension_first {
        InitializePausableRedemptionsBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .paused(false)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("first initialize should succeed");
        svm.expire_blockhash();
    }

    let err = InitializePausableRedemptionsBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .paused(false)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

#[test]
fn test_update_paused_true_blocks_redeem() {
    let (mut svm, authority, asset_mint, share_mint, user, vault_pubkey, user_share_account) =
        setup(Some(false));

    UpdatePausableRedemptionsBuilder::new()
        .authority(authority.pubkey())
        .vault(vault_pubkey)
        .paused(true)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("update to paused=true should succeed");

    let pausable_redempts = pausable_redemptions::get_state(
        svm.get_account(&vault_pubkey).expect("vault exists").data(),
    )
    .expect("PausableRedemptions should be initialized");
    assert!(pausable_redempts.paused);

    let err = create_redeem_request(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        user_share_account,
        SHARE_AMOUNT,
    )
    .unwrap_err();
    assert_error_code(&err, 6028, "RedemptionsPaused");
}

#[test_case(false, false, 6006, "UninitializedExtension" ; "without_init")]
#[test_case(true, true, 6001, "UnauthorizedSigner" ; "wrong_authority")]
fn test_update_pausable_redemptions_fails(
    init_extension: bool,
    use_wrong_signer: bool,
    expected_error: u32,
    expected_name: &str,
) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let (
        authority,
        _payer,
        _mint_authority,
        _asset_mint,
        _share_mint,
        user,
        _operator,
        _fee_recipient,
        _reserve_pubkey,
        vault_pubkey,
        _pending_vault_pubkey,
        _fee_recipient_ata,
        _user_share_account,
    ) = set_up_async_vault(&mut svm, token::ID, None, token::ID, 0);

    if init_extension {
        InitializePausableRedemptionsBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .paused(false)
            .instruction()
            .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
            .expect("initialize_pausable_redemptions should succeed");
    }

    let signer: &Keypair = if use_wrong_signer { &user } else { &authority };

    let err = UpdatePausableRedemptionsBuilder::new()
        .authority(signer.pubkey())
        .vault(vault_pubkey)
        .paused(true)
        .instruction()
        .send_transaction(&mut svm, &signer.pubkey(), &[signer])
        .unwrap_err();
    assert_error_code(&err, expected_error, expected_name);
}

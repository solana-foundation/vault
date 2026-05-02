use anchor_spl::token;
use async_vault_client::{
    lite::SendTransaction, sdk::program_id, FeeType, InitializeDepositFeeBuilder,
    InitializeWithdrawalFeeBuilder, UpdateDepositFeeBuilder, UpdateWithdrawalFeeBuilder, Vault,
};
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Keypair, signer::Signer};
use test_case::test_case;

use crate::helper_functions::{assert_error_code, set_up_async_vault};

#[derive(Clone, Copy)]
enum FeeKind {
    Deposit,
    Withdrawal,
}

fn init_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    fee: FeeType,
    kind: FeeKind,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    match kind {
        FeeKind::Deposit => InitializeDepositFeeBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .share_mint(share_mint)
            .vault(vault)
            .deposit_fee(fee)
            .instruction()
            .send_transaction(svm, &authority.pubkey(), &[authority]),
        FeeKind::Withdrawal => InitializeWithdrawalFeeBuilder::new()
            .payer(authority.pubkey())
            .authority(authority.pubkey())
            .share_mint(share_mint)
            .vault(vault)
            .withdrawal_fee(fee)
            .instruction()
            .send_transaction(svm, &authority.pubkey(), &[authority]),
    }
}

fn update_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    fee: FeeType,
    kind: FeeKind,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    match kind {
        FeeKind::Deposit => UpdateDepositFeeBuilder::new()
            .authority(authority.pubkey())
            .share_mint(share_mint)
            .vault(vault)
            .new_deposit_fee(fee)
            .instruction()
            .send_transaction(svm, &authority.pubkey(), &[authority]),
        FeeKind::Withdrawal => UpdateWithdrawalFeeBuilder::new()
            .authority(authority.pubkey())
            .share_mint(share_mint)
            .vault(vault)
            .new_withdrawal_fee(fee)
            .instruction()
            .send_transaction(svm, &authority.pubkey(), &[authority]),
    }
}

fn setup_vault() -> (LiteSVM, Keypair, Keypair, Pubkey) {
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
    (svm, authority, share_mint, vault_pubkey)
}

#[test_case(FeeKind::Deposit, FeeType::FixedAmount { amount: 100 }, FeeType::Percentage { bps: 500 } ; "deposit")]
#[test_case(FeeKind::Withdrawal, FeeType::Percentage { bps: 200 }, FeeType::FixedAmount { amount: 50 } ; "withdrawal")]
fn test_initialize_and_update_fee(kind: FeeKind, initial_fee: FeeType, updated_fee: FeeType) {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    init_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        initial_fee,
        kind,
    )
    .expect("init fee should succeed");

    let vault_account = svm.get_account(&vault_pubkey).unwrap();
    let vault_config = Vault::from_bytes(vault_account.data()).unwrap();
    assert!(!vault_config.initialized);

    update_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        updated_fee,
        kind,
    )
    .expect("update fee should succeed");
}

#[test]
fn test_initialize_both_fees() {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    InitializeDepositFeeBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .deposit_fee(deposit_fee)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("init deposit fee should succeed");

    let withdrawal_fee = FeeType::Percentage { bps: 300 };
    InitializeWithdrawalFeeBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .withdrawal_fee(withdrawal_fee)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("init withdrawal fee should succeed");
}

#[test_case(FeeKind::Deposit, FeeType::FixedAmount { amount: 100 } ; "deposit")]
#[test_case(FeeKind::Withdrawal, FeeType::Percentage { bps: 100 } ; "withdrawal")]
fn test_duplicate_init_fails(kind: FeeKind, fee: FeeType) {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    init_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        fee.clone(),
        kind,
    )
    .expect("first init should succeed");

    svm.expire_blockhash();

    let result = init_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        fee,
        kind,
    );
    assert_error_code(&result.unwrap_err(), 6005, "ExtensionAlreadyInitialized");
}

#[test_case(FeeKind::Deposit, FeeType::FixedAmount { amount: 100 } ; "deposit")]
#[test_case(FeeKind::Withdrawal, FeeType::Percentage { bps: 100 } ; "withdrawal")]
fn test_update_before_init_fails(kind: FeeKind, fee: FeeType) {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    let result = update_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        fee,
        kind,
    );
    assert_error_code(&result.unwrap_err(), 6006, "UninitializedExtension");
}

#[test_case(FeeKind::Deposit ; "deposit")]
#[test_case(FeeKind::Withdrawal ; "withdrawal")]
fn test_invalid_bps_init_fails(kind: FeeKind) {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    let fee = FeeType::Percentage { bps: 10_001 };
    let result = init_fee(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        fee,
        kind,
    );
    assert_error_code(&result.unwrap_err(), 6000, "FeeBPSLimitReached");
}

#[test]
fn test_initialize_fee_unauthorized_signer_fails() {
    let (mut svm, _authority, share_mint, vault_pubkey) = setup_vault();

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    let result = InitializeDepositFeeBuilder::new()
        .payer(unauthorized.pubkey())
        .authority(unauthorized.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .deposit_fee(deposit_fee)
        .instruction()
        .send_transaction(&mut svm, &unauthorized.pubkey(), &[&unauthorized]);
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

#[test]
fn test_update_fee_unauthorized_signer_fails() {
    let (mut svm, authority, share_mint, vault_pubkey) = setup_vault();

    let deposit_fee = FeeType::FixedAmount { amount: 100 };
    InitializeDepositFeeBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .deposit_fee(deposit_fee)
        .instruction()
        .send_transaction(&mut svm, &authority.pubkey(), &[&authority])
        .expect("init should succeed");

    let unauthorized = Keypair::new();
    svm.airdrop(&unauthorized.pubkey(), 1_000_000_000).unwrap();

    let new_fee = FeeType::FixedAmount { amount: 200 };
    let result = UpdateDepositFeeBuilder::new()
        .authority(unauthorized.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(vault_pubkey)
        .new_deposit_fee(new_fee)
        .instruction()
        .send_transaction(&mut svm, &unauthorized.pubkey(), &[&unauthorized]);
    assert_error_code(&result.unwrap_err(), 6001, "UnauthorizedSigner");
}

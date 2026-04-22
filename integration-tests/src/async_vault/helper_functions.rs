use anchor_spl::{
    token::{self, spl_token},
    token_2022::spl_token_2022,
};
use async_vault_client::{
    sdk::{program_id, IntoSdkInstruction as _},
    CreateVaultBuilder, FeeType, InitializeDepositFeeBuilder, InitializeWithdrawalFeeBuilder,
    UpdateDepositFeeBuilder, UpdateWithdrawalFeeBuilder, Vault,
};
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction::create_account, transaction::Transaction,
};

use crate::async_vault::constants::{PENDING_VAULT_SEED, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED};

pub fn async_vault_program_id() -> Pubkey {
    program_id()
}

pub fn create_mint(svm: &mut LiteSVM, signer: &Keypair, mint: &Keypair) {
    use spl_token_2022::state::Mint;
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);
    let init_account_ix = create_account(
        &signer.pubkey(),
        &mint.pubkey(),
        rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = spl_token_2022::instruction::initialize_mint(
        &spl_token::ID,
        &mint.pubkey(),
        &signer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let init_tx = Transaction::new_signed_with_payer(
        &[init_account_ix, init_mint_ix],
        Some(&signer.pubkey()),
        &[mint, signer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(init_tx)
        .expect("create_mint transaction failed");
}

pub fn create_async_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    payer: &Keypair,
    mint_authority: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    pending_vault: Pubkey,
    vault: Pubkey,
    initial_price: u64,
    async_inflows: bool,
    async_outflows: bool,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = CreateVaultBuilder::new()
        .payer(payer.pubkey())
        .mint_authority(mint_authority.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .pending_vault(pending_vault)
        .vault(vault)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .authority(authority.pubkey())
        .initial_price(initial_price)
        .async_inflows(async_inflows)
        .async_outflows(async_outflows)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        blockhash,
    );

    svm.send_transaction(tx)
}

pub type AsyncVaultAccount = Vault;

pub fn init_deposit_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    deposit_fee: FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeDepositFeeBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .share_mint(share_mint)
        .vault(vault)
        .deposit_fee(deposit_fee)
        .instruction()
        .into_sdk_instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}

pub fn init_withdrawal_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    withdrawal_fee: FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeWithdrawalFeeBuilder::new()
        .payer(authority.pubkey())
        .authority(authority.pubkey())
        .share_mint(share_mint)
        .vault(vault)
        .withdrawal_fee(withdrawal_fee)
        .instruction()
        .into_sdk_instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}

pub fn update_deposit_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    new_deposit_fee: FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateDepositFeeBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint)
        .vault(vault)
        .new_deposit_fee(new_deposit_fee)
        .instruction()
        .into_sdk_instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}

pub fn update_withdrawal_fee(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
    new_withdrawal_fee: FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateWithdrawalFeeBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint)
        .vault(vault)
        .new_withdrawal_fee(new_withdrawal_fee)
        .instruction()
        .into_sdk_instruction();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}

pub fn assert_error_code(
    tx_result: &FailedTransactionMetadata,
    expected_code: u32,
    error_name: &str,
) {
    let error_string = format!("{:?}", tx_result);
    assert!(
        error_string.contains(&format!("Custom({})", expected_code))
            || error_string.contains(error_name),
        "Expected error code {} ({}), got: {:?}",
        expected_code,
        error_name,
        error_string
    );
}

pub fn setup_vault(
    svm: &mut LiteSVM,
) -> (Keypair, Keypair, Keypair, Keypair, Pubkey, Pubkey, Pubkey) {
    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(svm, &mint_authority, &asset_mint);
    create_mint(svm, &mint_authority, &share_mint);

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
        svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
        100_000_000,
        true,
        true,
        token::ID,
        token::ID,
    )
    .expect("vault creation should succeed");

    (
        authority,
        payer,
        mint_authority,
        share_mint,
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
    )
}

pub fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(async_vault_program_id(), program_bytes)
        .unwrap();
    svm
}

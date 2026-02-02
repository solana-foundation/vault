use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_sdk::{
    signature::Keypair, signer::Signer, system_instruction::create_account,
    transaction::Transaction,
};
use vault_client::{
    sdk::IntoSdkInstruction, CloseVaultBuilder, CreateVaultBuilder, DonateAssetsBuilder, FeeType,
    Pubkey, UpdateVaultBuilder,
};

use anchor_spl::{
    associated_token::{
        get_associated_token_address_with_program_id,
        spl_associated_token_account::instruction::create_associated_token_account,
    },
    token::{self, spl_token, Mint},
    token_2022::spl_token_2022::{self},
};

pub fn create_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    payer: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    deposit_fees: FeeType,
    withdraw_fees: FeeType,
    vault_asset_cap: u64,
    initial_price: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = CreateVaultBuilder::new()
        .authority(authority.pubkey())
        .payer(payer.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .deposit_fees(deposit_fees)
        .withdraw_fees(withdraw_fees)
        .vault_asset_cap(vault_asset_cap)
        .initial_price(initial_price)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);

    return svm.send_transaction(tx);
}

pub fn close_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    payer: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = CloseVaultBuilder::new()
        .authority(authority.pubkey())
        .payer(payer.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .rent_destination(payer.pubkey())
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        blockhash,
    );

    return svm.send_transaction(tx);
}
pub fn update_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    vault: Pubkey,
    deposit_fees: FeeType,
    withdraw_fees: FeeType,
    vault_asset_cap: u64,
    paused: bool,
    new_authority: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateVaultBuilder::new()
        .authority(authority.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .vault(vault)
        .deposit_fees(deposit_fees)
        .withdraw_fees(withdraw_fees)
        .vault_asset_cap(vault_asset_cap)
        .paused(paused)
        .new_authority(new_authority)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );

    return svm.send_transaction(tx);
}

pub fn create_mint(svm: &mut LiteSVM, signer: &Keypair, mint: &Keypair) {
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);
    let init_account_ix: solana_sdk::instruction::Instruction = create_account(
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
        &[&mint, &signer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(init_tx)
        .expect("create_mint transaction failed");
}

pub fn donate_assets(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    authority_assets_account: Pubkey,
    assets_amount: u64,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = DonateAssetsBuilder::new()
        .authority(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .authority_assets_account(authority_assets_account)
        .assets(assets_amount)
        .token_program(token::ID)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    return svm.send_transaction(tx);
}

pub fn create_ata(svm: &mut LiteSVM, owner: &Keypair, mint: &Pubkey) -> Pubkey {
    let ata = get_associated_token_address_with_program_id(&owner.pubkey(), &mint, &spl_token::ID);

    let ata_init_ix =
        create_associated_token_account(&owner.pubkey(), &owner.pubkey(), &mint, &spl_token::ID);
    let init_tx = Transaction::new_signed_with_payer(
        &[ata_init_ix],
        Some(&owner.pubkey()),
        &[&owner],
        svm.latest_blockhash(),
    );
    svm.send_transaction(init_tx).unwrap();
    return ata;
}

pub fn helper_mint_to(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    // Create mint_to instruction
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        account,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    // Send transaction
    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Failed to mint tokens");
}

pub fn assert_error_code(
    tx_result: &litesvm::types::FailedTransactionMetadata,
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

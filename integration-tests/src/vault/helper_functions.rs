use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_sdk::{
    account::{Account, ReadableAccount},
    program_pack::Pack,
    signature::Keypair,
    signer::Signer,
    system_instruction::create_account,
    transaction::Transaction,
};
use vault_client::{
    sdk::IntoSdkInstruction, CloseVaultBuilder, CreateVaultBuilder, DepositBuilder, FeeType,
    MintBuilder, Pubkey, RedeemBuilder, UpdateVaultBuilder, WithdrawBuilder,
};

use anchor_spl::{
    associated_token::{
        get_associated_token_address_with_program_id,
        spl_associated_token_account::instruction::create_associated_token_account,
    },
    token::spl_token,
    token_2022::{
        self,
        spl_token_2022::{
            self,
            extension::{
                transfer_fee::instruction::initialize_transfer_fee_config, ExtensionType,
                StateWithExtensions,
            },
            state::Mint,
        },
    },
};
use spl_token::state::Account as TokenAccount;
use spl_token_2022::state::{Account as TokenAccount2022, Mint as Token2022Mint};

pub fn create_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    payer: &Keypair,
    mint_authority: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    deposit_fees: FeeType,
    withdraw_fees: FeeType,
    vault_asset_cap: u64,
    initial_price: u64,
    fee_recipient: Pubkey,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = CreateVaultBuilder::new()
        .authority(authority.pubkey())
        .mint_authority(mint_authority.pubkey())
        .payer(payer.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .deposit_fees(deposit_fees)
        .withdraw_fees(withdraw_fees)
        .vault_asset_cap(vault_asset_cap)
        .initial_price(initial_price)
        .fee_recipient(fee_recipient)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        blockhash,
    );

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

pub fn deposit(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    fee_recipient: Pubkey,
    user_assets_account: Pubkey,
    user_shares_account: Pubkey,
    assets_amount: u64,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = DepositBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .fee_recipient(fee_recipient)
        .user_assets_account(user_assets_account)
        .user_shares_account(user_shares_account)
        .assets(assets_amount)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    return svm.send_transaction(tx);
}

pub fn mint(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    fee_recipient: Pubkey,
    user_assets_account: Pubkey,
    user_shares_account: Pubkey,
    shares_amount: u64,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = MintBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .fee_recipient(fee_recipient)
        .user_assets_account(user_assets_account)
        .user_shares_account(user_shares_account)
        .shares(shares_amount)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    return svm.send_transaction(tx);
}

pub fn withdraw(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    fee_recipient: Pubkey,
    user_assets_account: Pubkey,
    user_shares_account: Pubkey,
    assets_amount: u64,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = WithdrawBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .fee_recipient(fee_recipient)
        .user_assets_account(user_assets_account)
        .user_shares_account(user_shares_account)
        .assets(assets_amount)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    return svm.send_transaction(tx);
}

pub fn redeem(
    svm: &mut LiteSVM,
    user: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
    fee_recipient: Pubkey,
    user_assets_account: Pubkey,
    user_shares_account: Pubkey,
    shares_amount: u64,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = RedeemBuilder::new()
        .user(user.pubkey())
        .asset_mint(asset_mint)
        .share_mint(share_mint)
        .reserve(reserve)
        .vault(vault)
        .fee_recipient(fee_recipient)
        .user_assets_account(user_assets_account)
        .user_shares_account(user_shares_account)
        .shares(shares_amount)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
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

pub fn create_ata(
    svm: &mut LiteSVM,
    owner: &Keypair,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Pubkey {
    let ata = get_associated_token_address_with_program_id(&owner.pubkey(), &mint, token_program);

    let ata_init_ix =
        create_associated_token_account(&owner.pubkey(), &owner.pubkey(), &mint, token_program);

    let init_tx = Transaction::new_signed_with_payer(
        &[ata_init_ix],
        Some(&owner.pubkey()),
        &[&owner],
        svm.latest_blockhash(),
    );
    svm.send_transaction(init_tx).unwrap();
    ata
}

pub fn helper_mint_to(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
    token_program: &Pubkey,
) {
    let mint_to_ix = spl_token_2022::instruction::mint_to(
        token_program,
        mint,
        account,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

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

pub fn get_fee(fee: FeeType, total_amount: u64) -> u64 {
    match fee {
        FeeType::Percentage { bps } => {
            let fee = total_amount
                .checked_mul(bps.into())
                .expect("overflow")
                .checked_add(9_999)
                .expect("overflow")
                .checked_div(10_000)
                .expect("overflow");
            return fee;
        }
        FeeType::FixedAmount { amount } => return amount,
        FeeType::NoFee => return 0,
    }
}

pub fn set_up_vault(
    svm: &mut LiteSVM,
    mint_authority: Keypair,
    asset_mint: &Keypair,
    share_mint: &Keypair,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
    deposit_fees: &FeeType,
    withdraw_fees: &FeeType,
) -> (Keypair, Keypair, Keypair, Keypair, Keypair, Pubkey, Pubkey) {
    let authority = Keypair::new();
    let user = Keypair::new();
    let payer = Keypair::new();

    let fee_recipient = Keypair::new();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

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
    let result = create_vault(
        svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        deposit_fees.clone(),
        withdraw_fees.clone(),
        100_000_000,
        1,
        fee_recipient.pubkey(),
        asset_token_program,
        share_token_program,
    );
    let _ = update_vault(
        svm,
        &authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        deposit_fees.clone(),
        withdraw_fees.clone(),
        100_000_000,
        false,
        authority.pubkey(),
    );
    return (
        authority,
        user,
        payer,
        mint_authority,
        fee_recipient,
        reserve_pubkey,
        vault_pubkey,
    );
}

pub fn create_mint_with_transfer_fee(
    svm: &mut LiteSVM,
    signer: &Keypair,
    mint: &Keypair,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) {
    // Calculate space needed for mint + transfer fee extension
    let space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
            .unwrap();

    let rent = svm.minimum_balance_for_rent_exemption(space);

    // Create account with proper space
    let create_account_ix = create_account(
        &signer.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token_2022::id(),
    );

    // Initialize transfer fee extension BEFORE initializing mint
    let init_transfer_fee_ix = initialize_transfer_fee_config(
        &spl_token_2022::id(),
        &mint.pubkey(),
        Some(&signer.pubkey()), // transfer_fee_config_authority
        Some(&signer.pubkey()), // withdraw_withheld_authority
        transfer_fee_basis_points,
        maximum_fee,
    )
    .unwrap();

    // Initialize the mint (this must come AFTER extension initialization)
    let init_mint_ix = spl_token_2022::instruction::initialize_mint(
        &spl_token_2022::id(),
        &mint.pubkey(),
        &signer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_transfer_fee_ix, init_mint_ix],
        Some(&signer.pubkey()),
        &[&mint, &signer],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("create_mint_with_transfer_fee transaction failed");
}

/// gets the amount of a token account, depending on the account owner
pub fn get_token_account_amount(account: &Account) -> u64 {
    if account.owner == token_2022::ID {
        StateWithExtensions::<TokenAccount2022>::unpack(account.data())
            .unwrap()
            .base
            .amount
    } else {
        TokenAccount::unpack(account.data()).unwrap().amount
    }
}

/// gets the supply of a token mint, depending on the account owner
pub fn get_mint_supply(account: &Account) -> u64 {
    if account.owner == token_2022::ID {
        let state = StateWithExtensions::<Token2022Mint>::unpack(account.data())
            .expect("unpack token-2022 mint");
        state.base.supply
    } else {
        spl_token::state::Mint::unpack(account.data())
            .expect("unpack token-keg mint")
            .supply
    }
}

fn transfer_fee_from_params(amount: u64, bps: u16, max_fee: u64) -> u64 {
    if amount == 0 || bps == 0 {
        return 0;
    }
    let numerator = (amount as u128) * (bps as u128);
    let fee = (numerator + 10_000u128 - 1) / 10_000u128; // ceil
    let fee_u64 = u64::try_from(fee).expect("fee overflow u64");
    fee_u64.min(max_fee)
}

/// calculates the amount to receive after transfer fees (from token2022) are substracted
pub fn recv_amount_from_params(amount: u64, bps: u16, max_fee: u64) -> u64 {
    amount.saturating_sub(transfer_fee_from_params(amount, bps, max_fee))
}

pub fn get_assets_from_shares(total_assets: u64, supply: u64, share_amount: u64) -> u64 {
    let numerator = u128::from(share_amount)
        .checked_mul(u128::from(total_assets))
        .unwrap();

    let denominator = u128::from(supply.checked_add(1).unwrap());

    return numerator
        .checked_div(denominator)
        .unwrap()
        .try_into()
        .unwrap();
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

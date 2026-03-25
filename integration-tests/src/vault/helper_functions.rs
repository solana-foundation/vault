use hook_client::InitializeDepositExtraMetaAccountsBuilder;
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
    system_program,
    transaction::Transaction,
};
use vault_client::{
    sdk::IntoSdkInstruction as _, CloseVaultBuilder, CreateVaultBuilder, DepositBuilder, FeeType,
    InitializeDepositFeesBuilder, InitializeDepositHookBuilder, InitializeVaultBuilder,
    InitializeWithdrawalFeesBuilder, MintBuilder, Pubkey, RedeemBuilder, UpdateDepositFeesBuilder,
    UpdateVaultBuilder, UpdateWithdrawalFeesBuilder, VaultConfig, WithdrawBuilder,
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
    share_mint: Pubkey,
    reserve: Pubkey,
    vault: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = CloseVaultBuilder::new()
        .authority(authority.pubkey())
        .payer(payer.pubkey())
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
    share_mint: Pubkey,
    vault: Pubkey,
    vault_asset_cap: u64,
    paused: bool,
    new_authority: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateVaultBuilder::new()
        .authority(authority.pubkey())
        .share_mint(share_mint)
        .vault(vault)
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
    min_shares: u64,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
    hook_program: Pubkey,
    protocol: Option<Pubkey>,
    nav_return_data: Option<Pubkey>,
    instructions: Option<Pubkey>,
    extra_metas: Option<Pubkey>,
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
        .min_shares(min_shares)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .hook_program(Some(hook_program))
        .protocol(protocol)
        .extra_metas(extra_metas)
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
    max_assets: u64,
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
        .max_assets(max_assets)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .hook_program(Some(system_program::ID))
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
    max_shares: u64,
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
        .max_shares(max_shares)
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
    min_assets: u64,
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
        .min_assets(min_assets)
        .asset_token_program(asset_token_program)
        .share_token_program(share_token_program)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    return svm.send_transaction(tx);
}

pub fn init_deposit_fees(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    deposit_fee: &FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeDepositFeesBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
        .deposit_fee(deposit_fee.clone())
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

pub fn update_deposit_fees(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    new_deposit_fee: &FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateDepositFeesBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
        .new_deposit_fee(new_deposit_fee.clone())
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

pub fn init_withdrawal_fees(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    withdrawal_fee: &FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeWithdrawalFeesBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
        .withdrawal_fee(withdrawal_fee.clone())
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

pub fn update_withdrawal_fees(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    new_withdrawal_fee: &FeeType,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = UpdateWithdrawalFeesBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
        .new_withdrawal_fee(new_withdrawal_fee.clone())
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

pub fn init_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeVaultBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
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

pub fn get_fee(fee: Option<FeeType>, total_amount: u64) -> u64 {
    match fee {
        Some(FeeType::Percentage { bps }) => {
            let fee = total_amount
                .checked_mul(bps.into())
                .expect("overflow")
                .checked_add(9_999)
                .expect("overflow")
                .checked_div(10_000)
                .expect("overflow");
            return fee;
        }
        Some(FeeType::FixedAmount { amount }) => return amount,
        None => 0,
    }
}

pub fn set_up_vault(
    svm: &mut LiteSVM,
    mint_authority: Keypair,
    asset_mint: &Keypair,
    share_mint: &Keypair,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
    deposit_fee_opt: Option<FeeType>,
    withdrawal_fee_opt: Option<FeeType>,
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
        &[b"reserve", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &vault_client::sdk::program_id(),
    );
    create_vault(
        svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        100_000_000,
        1,
        fee_recipient.pubkey(),
        asset_token_program,
        share_token_program,
    )
    .expect("Failed to create vault");

    let _ = update_vault(
        svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        100_000_000,
        false,
        authority.pubkey(),
    );

    match deposit_fee_opt {
        Some(deposit_fee) => {
            init_deposit_fees(
                svm,
                &authority,
                &share_mint.pubkey(),
                &vault_pubkey,
                &deposit_fee,
            )
            .expect("Failed to initialize deposit fees");
        }
        None => (),
    }

    match withdrawal_fee_opt {
        Some(withdrawal_fee) => {
            init_withdrawal_fees(
                svm,
                &authority,
                &share_mint.pubkey(),
                &vault_pubkey,
                &withdrawal_fee,
            )
            .expect("Failed to initialize withdrawal fees");
        }
        None => (),
    }

    init_vault(svm, &authority, &share_mint.pubkey(), &vault_pubkey)
        .expect("Failed to initialize vault");

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

pub fn init_deposit_hook(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    hook_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeDepositHookBuilder::new()
        .authority(authority.pubkey())
        .share_mint(*share_mint)
        .vault(*vault)
        .hook_program(hook_program)
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

pub fn init_withdraw_hook(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: &Pubkey,
    vault: &Pubkey,
    hook_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    // Build initialize_withdraw_hook instruction using the Anchor discriminator computed at runtime
    let disc = {
        let hash = solana_sdk::hash::hash(b"global:initialize_withdraw_hook");
        let b = hash.to_bytes();
        [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
    };
    let mut data = disc.to_vec();
    data.extend_from_slice(hook_program.as_ref());

    let ix = solana_sdk::instruction::Instruction {
        program_id: vault_client::sdk::program_id(),
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new_readonly(authority.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new_readonly(*share_mint, false),
            solana_sdk::instruction::AccountMeta::new(*vault, false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        blockhash,
    );

    return svm.send_transaction(tx);
}

pub fn init_withdraw_extra_meta_accounts(
    svm: &mut LiteSVM,
    payer: &Keypair,
    asset_mint: &Pubkey,
    share_mint: &Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let (extra_metas, _) = Pubkey::find_program_address(
        &[b"extra_account_metas", b"withdraw", share_mint.as_ref()],
        &hook_client::sdk::program_id(),
    );

    let disc = {
        let hash = solana_sdk::hash::hash(b"global:initialize_withdraw_extra_meta_accounts");
        let b = hash.to_bytes();
        [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: hook_client::sdk::program_id(),
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new_readonly(*asset_mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(*share_mint, false),
            solana_sdk::instruction::AccountMeta::new(extra_metas, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                anchor_spl::token::spl_token::id(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: disc.to_vec(),
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);

    return svm.send_transaction(tx);
}

pub fn init_deposit_extra_meta_accounts(
    svm: &mut LiteSVM,
    payer: &Keypair,
    asset_mint: &Pubkey,
    share_mint: &Pubkey,
    _vault: &Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let (extra_metas, _) = Pubkey::find_program_address(
        &[b"extra_account_metas", b"deposit", share_mint.as_ref()],
        &hook_client::sdk::program_id(),
    );

    let ix = InitializeDepositExtraMetaAccountsBuilder::new()
        .payer(payer.pubkey())
        .asset_mint(*asset_mint)
        .share_mint_address(*share_mint)
        .extra_metas(extra_metas)
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);

    return svm.send_transaction(tx);
}

/// gets the assets held in VaultConfig's reserve account
pub fn get_vault_asset_balance(svm: &LiteSVM, vault_pubkey: &Pubkey) -> u64 {
    let vault = svm
        .get_account(&vault_pubkey)
        .expect("Vault account should exist");
    let vault_config = VaultConfig::from_bytes(vault.data()).unwrap();

    let reserve_acc = svm
        .get_account(&vault_config.vault_token_account)
        .expect("could not fetch vault_token_account");
    get_token_account_amount(&reserve_acc)
}

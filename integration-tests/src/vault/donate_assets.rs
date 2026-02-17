use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::Account as TokenAccount;
use vault_client::{sdk::program_id, FeeType, Pubkey};

use crate::vault::helper_functions::{
    create_ata, create_mint, create_vault, donate_assets, helper_mint_to, update_vault,
};
use test_case::test_case;

#[test_case(FeeType::NoFee, FeeType::NoFee,  FeeType::Percentage { bps: 100 },FeeType::Percentage { bps: 0 }, false, 100_000_000; "Donate assets successfully")]
fn test_donate_assets_to_vault(
    deposit_fee: FeeType,
    withdraw_fee: FeeType,
    updated_deposit_fee: FeeType,
    updated_withdraw_fee: FeeType,
    updated_paused_status: bool,
    updated_vault_asset_cap: u64,
) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);
    let authority = Keypair::new();
    let user = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);
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

    let _ = create_vault(
        &mut svm,
        &authority,
        &payer,
        &mint_authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        deposit_fee.clone(),
        withdraw_fee.clone(),
        0,
        100_000,
        fee_recipient.pubkey(),
        token::ID,
        token::ID,
    );

    let _ = update_vault(
        &mut svm,
        &authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        vault_pubkey,
        updated_deposit_fee.clone(),
        updated_withdraw_fee.clone(),
        updated_vault_asset_cap,
        updated_paused_status,
        authority.pubkey(),
    );

    let authority_asset_ata = create_ata(&mut svm, &authority, &asset_mint.pubkey(), &token::ID);
    let authority_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &authority_asset_ata,
        &mint_authority,
        authority_asset_amount,
        &token::ID,
    );

    let mut reserve_ata_account = svm
        .get_account(&reserve_pubkey)
        .expect("Vault account should exist");

    let mut reserve_ata_balance_before = TokenAccount::unpack(reserve_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(reserve_ata_balance_before, 0);

    let mut authority_asset_ata_account = svm
        .get_account(&authority_asset_ata)
        .expect("Vault account should exist");

    let mut authority_asset_balance_before =
        TokenAccount::unpack(authority_asset_ata_account.data())
            .unwrap()
            .amount;
    assert_eq!(authority_asset_balance_before, authority_asset_amount);

    let deposit_amount = 500_000;

    let result = donate_assets(
        &mut svm,
        &authority,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        authority_asset_ata,
        deposit_amount,
    );
    assert_eq!(result.is_ok(), true, "Unexpected result for test case");
    reserve_ata_account = svm
        .get_account(&reserve_pubkey)
        .expect("Vault account should exist");

    reserve_ata_balance_before = TokenAccount::unpack(reserve_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(reserve_ata_balance_before, deposit_amount);
}

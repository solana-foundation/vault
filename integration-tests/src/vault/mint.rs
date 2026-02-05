use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::Account as TokenAccount;
use vault_client::{sdk::program_id, FeeType, Pubkey};

use crate::vault::helper_functions::{
    create_ata, create_mint, create_vault, helper_mint_to, mint, update_vault,
};
use test_case::test_case;

#[test_case(FeeType::NoFee, FeeType::NoFee,  FeeType::Percentage { bps: 100 },FeeType::Percentage { bps: 0 }, false, 100_000; "Mint successfully")]
fn test_mint_vault(
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
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();

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
        100_000,
        1,
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
    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID,
    );
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    let mut fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Vault account should exist");

    let fee_recipient_balance_before = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(fee_recipient_balance_before, 0);

    let mut user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("Vault account should exist");

    let user_asset_balance_before = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let mut user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("Vault account should exist");

    let user_share_balance_before = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_before, 0);
    let mint_amount = 500_000;
    let result = mint(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        mint_amount,
        token::ID,
        token::ID,
    );

    assert_eq!(result.is_ok(), true, "Unexpected result for test case");

    fee_recipient_ata_account = svm
        .get_account(&fee_recipient_ata)
        .expect("Vault account should exist");

    let fee_recipient_balance_after = TokenAccount::unpack(fee_recipient_ata_account.data())
        .unwrap()
        .amount;
    let fee = 4999;
    assert_eq!(fee_recipient_balance_after, fee);

    user_asset_ata_account = svm
        .get_account(&user_asset_ata)
        .expect("Vault account should exist");

    let user_asset_balance_after = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;

    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(mint_amount)
            .expect("overflow")
            .checked_sub(fee)
            .expect("overflow")
    );

    user_share_ata_account = svm
        .get_account(&user_share_ata)
        .expect("Vault account should exist");

    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(user_share_balance_after, mint_amount);
}

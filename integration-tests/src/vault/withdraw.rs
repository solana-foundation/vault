use anchor_spl::token;
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, signature::Keypair, signer::Signer,
};
use spl_token::state::{Account as TokenAccount, Mint as TokenMint};
use vault_client::{sdk::program_id, FeeType};

use crate::vault::helper_functions::{
    create_ata, create_mint, deposit, get_fee, helper_mint_to, set_up_vault, withdraw
};
use test_case::test_case;

#[test_case(
    FeeType::Percentage { bps: 100 },  // 1% deposit fee
    FeeType::Percentage { bps: 50 };   // 0.5% withdraw fee
    "Withdraw successfully (percentage fees)"
)]
#[test_case(
    FeeType::NoFee,
    FeeType::NoFee;
    "Withdraw successfully (no fees)"
)]
fn test_withdraw_vault(
    deposit_fee: FeeType,
    withdraw_fee: FeeType,
) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes);

    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let mint_authority = Keypair::new();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);

    let (
        _, 
        user,
        _, 
        mint_authority, 
        fee_recipient, 
        reserve_pubkey, 
        vault_pubkey
    ) = set_up_vault(
        &mut svm,
        mint_authority,
        &asset_mint,
        &share_mint,
        token::ID,
        token::ID,
        &deposit_fee,
        &withdraw_fee,
    );

    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(),  &token::ID);

    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID
    );

    // -------------------- balances before deposit --------------------
    let fee_recipient_balance_before = TokenAccount::unpack(
        svm.get_account(&fee_recipient_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(fee_recipient_balance_before, 0);

    let user_asset_balance_before = TokenAccount::unpack(
        svm.get_account(&user_asset_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(user_asset_balance_before, user_asset_amount);

    let user_share_balance_before = TokenAccount::unpack(
        svm.get_account(&user_share_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(user_share_balance_before, 0);

    let reserve_balance_before = TokenAccount::unpack(
        svm.get_account(&reserve_pubkey).unwrap().data(),
    )
    .unwrap()
    .amount;
    // newly created reserve ATA should have 0 assets
    assert_eq!(reserve_balance_before, 0);

    let share_supply_before_deposit = TokenMint::unpack(
        svm.get_account(&share_mint.pubkey()).unwrap().data(),
    )
    .unwrap()
    .supply;
    assert_eq!(share_supply_before_deposit, 0);

    // -------------------- deposit --------------------
    let deposit_amount = 500_000;

    let result = deposit(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        deposit_amount,
        token::ID,
        token::ID,
    );
    assert!(result.is_ok(), "deposit failed unexpectedly");

    let deposit_fee_amount = get_fee(deposit_fee.clone(), deposit_amount);
    let deposit_net = deposit_amount.checked_sub(deposit_fee_amount).expect("overflow");

    // After deposit:
    // - fee recipient got deposit_fee_amount
    // - reserve got deposit_net
    // - user assets decreased by deposit_amount
    // - user shares minted = deposit_net
    let fee_recipient_after_deposit = TokenAccount::unpack(
        svm.get_account(&fee_recipient_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(fee_recipient_after_deposit, deposit_fee_amount);

    let reserve_after_deposit = TokenAccount::unpack(
        svm.get_account(&reserve_pubkey).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(reserve_after_deposit, deposit_net);

    let user_assets_after_deposit = TokenAccount::unpack(
        svm.get_account(&user_asset_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(user_assets_after_deposit, user_asset_amount - deposit_amount);

    let user_shares_after_deposit = TokenAccount::unpack(
        svm.get_account(&user_share_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(user_shares_after_deposit, deposit_net);

    let share_supply_after_deposit = TokenMint::unpack(
        svm.get_account(&share_mint.pubkey()).unwrap().data(),
    )
    .unwrap()
    .supply;
    assert_eq!(share_supply_after_deposit, deposit_net);

    // -------------------- withdraw --------------------
    // `assets_out` is NET to user. Vault additionally pays `withdraw_fee(assets_out)` out of reserve.
    // shares burned should cover gross = assets_out + fee.
    
    // assets_out small enough so gross <= user_shares_after_deposit.
    let assets_out: u64 = 100_000;

    let withdraw_fee_amount = get_fee(withdraw_fee.clone(), assets_out);
    let gross_amount = assets_out.checked_add(withdraw_fee_amount).expect("overflow");

    let result = withdraw(
        &mut svm,
        &user,
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        vault_pubkey,
        fee_recipient_ata,
        user_asset_ata,
        user_share_ata,
        assets_out, // NET to user
    );
    assert!(result.is_ok(), "withdraw failed unexpectedly");

    // -------------------- assert post-withdraw --------------------
    // fee recipient total = deposit_fee + withdraw_fee
    let fee_recipient_after_withdraw = TokenAccount::unpack(
        svm.get_account(&fee_recipient_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(
        fee_recipient_after_withdraw,
        deposit_fee_amount + withdraw_fee_amount
    );

    // user assets: -deposit_amount + assets_out
    let user_assets_after_withdraw = TokenAccount::unpack(
        svm.get_account(&user_asset_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(
        user_assets_after_withdraw,
        (user_asset_amount - deposit_amount) + assets_out
    );

    // reserve: +deposit_net - gross
    let reserve_after_withdraw = TokenAccount::unpack(
        svm.get_account(&reserve_pubkey).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(reserve_after_withdraw, deposit_net - gross_amount);

    // user shares: +deposit_net - gross
    let user_shares_after_withdraw = TokenAccount::unpack(
        svm.get_account(&user_share_ata).unwrap().data(),
    )
    .unwrap()
    .amount;
    assert_eq!(user_shares_after_withdraw, deposit_net - gross_amount);

    // share supply: deposit_net - gross
    let share_supply_after_withdraw = TokenMint::unpack(
        svm.get_account(&share_mint.pubkey()).unwrap().data(),
    )
    .unwrap()
    .supply;
    assert_eq!(share_supply_after_withdraw, deposit_net - gross_amount);
    
}

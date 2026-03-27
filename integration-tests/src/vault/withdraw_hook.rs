use anchor_spl::token::{self};
use dummy_client::{
    sdk::{program_id as dummy_program_id, IntoSdkInstruction as DummyIntoSdkInstruction},
    CreateVaultBuilder as DummyCreateVaultBuilder,
};
use hook_client::{
    sdk::program_id as hook_program_id, AddAssociatedProtocolBuilder,
    InitVaultAssociatedProtocolsBuilder, HOOK_PROGRAM_ID,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};
use spl_token::state::Account as TokenAccount;
use vault_client::{sdk::program_id, VaultConfig, WithdrawBuilder};

use crate::vault::helper_functions::{
    create_ata, create_mint, create_vault, deposit, get_vault_asset_balance, helper_mint_to,
    init_vault, init_withdraw_extra_meta_accounts, init_withdraw_hook, update_vault,
};

#[test]
fn test_withdraw_with_hook() {
    let mut svm = LiteSVM::new();

    let vault_program_bytes = include_bytes!("../../../target/deploy/vault.so");
    svm.add_program(program_id(), vault_program_bytes);

    let hook_program_bytes = include_bytes!("../../../target/deploy/hook_program.so");
    svm.add_program(HOOK_PROGRAM_ID, hook_program_bytes);

    let dummy_program_bytes = include_bytes!("../../../target/deploy/dummy_protocol.so");
    svm.add_program(dummy_program_id(), dummy_program_bytes);

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let fee_recipient = Keypair::new();
    let user = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);

    let (reserve_pubkey, _) =
        Pubkey::find_program_address(&[b"reserve", share_mint.pubkey().as_ref()], &program_id());
    let (vault_pubkey, _) =
        Pubkey::find_program_address(&[b"vault", share_mint.pubkey().as_ref()], &program_id());

    // Create the vault
    create_vault(
        &mut svm,
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
        token::ID,
        token::ID,
    )
    .expect("vault creation failed");

    update_vault(
        &mut svm,
        &authority,
        share_mint.pubkey(),
        vault_pubkey,
        100_000_000,
        false,
        authority.pubkey(),
    )
    .expect("vault update failed");

    // Add the withdraw hook extension
    init_withdraw_hook(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        HOOK_PROGRAM_ID,
    )
    .expect("init withdraw hook failed");

    // Initialize the extra meta accounts for the withdraw hook
    init_withdraw_extra_meta_accounts(
        &mut svm,
        &authority,
        &asset_mint.pubkey(),
        &share_mint.pubkey(),
    )
    .expect("init withdraw extra meta accounts failed");

    // Assert the extension was added
    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("vault account should exist");
    let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();

    // Initialize the vault
    init_vault(&mut svm, &authority, &share_mint.pubkey(), &vault_pubkey)
        .expect("init vault failed");

    // Create the dummy protocol vault
    let (dummy_vault_pubkey, _) = Pubkey::find_program_address(
        &[b"vault", share_mint.pubkey().as_ref()],
        &dummy_program_id(),
    );
    let dummy_create_vault_ix = DummyCreateVaultBuilder::new()
        .payer(payer.pubkey())
        .mint_authority(mint_authority.pubkey())
        .asset_mint(asset_mint.pubkey())
        .share_mint(share_mint.pubkey())
        .vault(dummy_vault_pubkey)
        .asset_token_program(token::ID)
        .share_token_program(token::ID)
        .instruction();
    let dummy_create_vault_ix =
        DummyIntoSdkInstruction::into_sdk_instruction(dummy_create_vault_ix);
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[dummy_create_vault_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        blockhash,
    );
    svm.send_transaction(tx).expect("dummy create vault failed");

    // Derive vault associated protocols PDA
    let (vault_associated_protocols_pubkey, _) = Pubkey::find_program_address(
        &[
            b"vault_associated_protocols",
            &vault_config.share_mint_address.to_bytes(),
        ],
        &hook_program_id(),
    );

    // Initialize vault associated protocols
    let init_vap_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        InitVaultAssociatedProtocolsBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .instruction(),
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[init_vap_ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );
    svm.send_transaction(tx)
        .expect("init vault associated protocols failed");

    // Derive associated_protocol PDAs
    let (vault_associated_protocol_pubkey, _) = Pubkey::find_program_address(
        &[
            b"vault_protocol_deposit",
            share_mint.pubkey().as_ref(),
            program_id().as_ref(),
        ],
        &hook_program_id(),
    );
    let (dummy_associated_protocol_pubkey, _) = Pubkey::find_program_address(
        &[
            b"vault_protocol_deposit",
            share_mint.pubkey().as_ref(),
            dummy_program_id().as_ref(),
        ],
        &hook_program_id(),
    );

    // Add vault program as associated protocol
    let add_vault_protocol_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        AddAssociatedProtocolBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .protocol(program_id())
            .associated_protocol(vault_associated_protocol_pubkey)
            .token_account(reserve_pubkey)
            .instruction(),
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[add_vault_protocol_ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );
    svm.send_transaction(tx)
        .expect("add vault program as protocol failed");

    // Add dummy program as associated protocol
    let add_protocol_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        AddAssociatedProtocolBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .protocol(dummy_program_id())
            .associated_protocol(dummy_associated_protocol_pubkey)
            .token_account(dummy_vault_pubkey)
            .instruction(),
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[add_protocol_ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );
    svm.send_transaction(tx)
        .expect("add associated protocol failed");

    let fee_recipient_ata = create_ata(&mut svm, &fee_recipient, &asset_mint.pubkey(), &token::ID);
    let user_asset_ata = create_ata(&mut svm, &user, &asset_mint.pubkey(), &token::ID);
    let user_share_ata = create_ata(&mut svm, &user, &share_mint.pubkey(), &token::ID);

    let user_asset_amount = 100_000_000;
    helper_mint_to(
        &mut svm,
        &asset_mint.pubkey(),
        &user_asset_ata,
        &mint_authority,
        user_asset_amount,
        &token::ID,
    );

    // Deposit without hook (vault has no deposit hook registered)
    let deposit_amount = 500_000u64;
    deposit(
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
        0,
        token::ID,
        token::ID,
        HOOK_PROGRAM_ID,
        None,
        None,
        None,
        None,
    )
    .expect("deposit failed");

    let reserve_balance_after_deposit =
        TokenAccount::unpack(svm.get_account(&reserve_pubkey).unwrap().data())
            .unwrap()
            .amount;
    assert_eq!(reserve_balance_after_deposit, deposit_amount);

    let user_shares_after_deposit =
        TokenAccount::unpack(svm.get_account(&user_share_ata).unwrap().data())
            .unwrap()
            .amount;
    assert!(user_shares_after_deposit > 0);

    let withdraw_amount = 100_000u64;
    let (withdraw_extra_meta_pubkey, _) = Pubkey::find_program_address(
        &[
            b"extra_account_metas",
            b"withdraw",
            share_mint.pubkey().as_ref(),
        ],
        &hook_program_id(),
    );

    let mut ix = vault_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        WithdrawBuilder::new()
            .user(user.pubkey())
            .asset_mint(asset_mint.pubkey())
            .share_mint(share_mint.pubkey())
            .reserve(reserve_pubkey)
            .vault(vault_pubkey)
            .fee_recipient(fee_recipient_ata)
            .extra_metas(Some(withdraw_extra_meta_pubkey))
            .user_assets_account(user_asset_ata)
            .user_shares_account(user_share_ata)
            .asset_token_program(token::ID)
            .share_token_program(token::ID)
            .hook_program(Some(HOOK_PROGRAM_ID))
            .protocol(Some(dummy_program_id()))
            .assets(withdraw_amount)
            .max_shares(u64::MAX)
            .instruction(),
    );

    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            vault_associated_protocols_pubkey,
            false,
        ));
    ix.accounts.push(solana_sdk::instruction::AccountMeta::new(
        dummy_vault_pubkey,
        false,
    ));
    // Associated-protocol PDAs and their token accounts needed by get_nav.
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            vault_associated_protocol_pubkey,
            false,
        ));
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            reserve_pubkey,
            false,
        ));
    ix.accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            dummy_associated_protocol_pubkey,
            false,
        ));

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "withdraw with hook failed: {:?}",
        result.err()
    );

    let user_asset_ata_account = svm.get_account(&user_asset_ata).unwrap();
    let user_asset_balance_after = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(
        user_asset_balance_after,
        user_asset_amount
            .checked_sub(deposit_amount)
            .unwrap()
            .checked_add(withdraw_amount)
            .unwrap()
    );

    let user_share_ata_account = svm.get_account(&user_share_ata).unwrap();
    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(
        user_share_balance_after,
        user_shares_after_deposit
            .checked_sub(withdraw_amount)
            .unwrap()
    );

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_amount - withdraw_amount);
}

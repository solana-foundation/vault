use anchor_spl::token::{self, spl_token::solana_program::system_program};
use dummy_client::{
    sdk::{program_id as dummy_program_id, IntoSdkInstruction as DummyIntoSdkInstruction},
    CreateVaultBuilder as DummyCreateVaultBuilder,
};
use hook_client::{
    sdk::program_id as hook_program_id, AddAssociatedProtocolBuilder,
    InitVaultAssociatedProtocolsBuilder, NavReturnData, UpdateNavBuilder, HOOK_PROGRAM_ID,
};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};
use spl_token::state::Account as TokenAccount;
use vault_client::{sdk::program_id, DepositBuilder, VaultConfig, VaultExtension};

use crate::vault::helper_functions::{
    create_ata, create_mint, create_vault, get_vault_asset_balance, helper_mint_to,
    init_deposit_extra_meta_accounts, init_deposit_hook, init_vault, update_vault,
};

#[test]
fn test_deposit_with_hook() {
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

    // Add the deposit hook extension
    init_deposit_hook(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        HOOK_PROGRAM_ID,
    )
    .expect("init deposit hook failed");

    // Initialize the extra meta accounts for the hook
    init_deposit_extra_meta_accounts(
        &mut svm,
        &authority,
        &asset_mint.pubkey(),
        &share_mint.pubkey(),
        &vault_pubkey,
    )
    .expect("init deposit extra meta accounts failed");

    // Assert the extension was added
    let vault_account = svm
        .get_account(&vault_pubkey)
        .expect("vault account should exist");
    let vault_config = VaultConfig::from_bytes(vault_account.data()).unwrap();
    assert_eq!(
        vault_config.extensions[0],
        VaultExtension::DepositHook(HOOK_PROGRAM_ID)
    );

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
        &[b"vault_associated_protocols", vault_pubkey.as_ref()],
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

    // Add dummy program as associated protocol
    let add_protocol_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        AddAssociatedProtocolBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .protocol(dummy_program_id())
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

    let reserve_account = svm.get_account(&reserve_pubkey).unwrap();
    let reserve_balance_before = TokenAccount::unpack(reserve_account.data()).unwrap().amount;
    assert_eq!(reserve_balance_before, 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, 0);

    let deposit_amount = 500_000;
    let (extra_meta_pubkey, _) = Pubkey::find_program_address(
        &[
            b"extra_account_metas",
            b"deposit",
            share_mint.pubkey().as_ref(),
        ],
        &hook_program_id(),
    );

    let (nav_return_data_pubkey, _) = Pubkey::find_program_address(
        &[b"vault_nav_data", vault_pubkey.as_ref()],
        &hook_program_id(),
    );

    let update_nav_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        UpdateNavBuilder::new()
            .payer(user.pubkey())
            .vault(vault_pubkey)
            .associated_protocols_info(vault_associated_protocols_pubkey)
            .nav_return_data(nav_return_data_pubkey)
            .instruction(),
    );

    let mut ix = vault_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        DepositBuilder::new()
            .user(user.pubkey())
            .asset_mint(asset_mint.pubkey())
            .share_mint(share_mint.pubkey())
            .reserve(reserve_pubkey)
            .vault(vault_pubkey)
            .fee_recipient(fee_recipient_ata)
            .extra_metas(Some(extra_meta_pubkey))
            .user_assets_account(user_asset_ata)
            .user_shares_account(user_share_ata)
            .asset_token_program(token::ID)
            .share_token_program(token::ID)
            .hook_program(Some(HOOK_PROGRAM_ID))
            .protocol(Some(dummy_program_id()))
            .nav_return_data(Some(nav_return_data_pubkey))
            .instructions(Some(solana_sdk::sysvar::instructions::ID))
            .assets(deposit_amount)
            .min_shares(0)
            .instruction(),
    );

    ix.accounts.push(solana_sdk::instruction::AccountMeta::new(
        dummy_vault_pubkey,
        false,
    ));

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[update_nav_ix, ix],
        Some(&user.pubkey()),
        &[&user],
        blockhash,
    );
    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "deposit with hook failed: {:?}",
        result.err()
    );

    let user_asset_ata_account = svm.get_account(&user_asset_ata).unwrap();
    let user_asset_balance_after = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(
        user_asset_balance_after,
        user_asset_amount.checked_sub(deposit_amount).unwrap()
    );

    let user_share_ata_account = svm.get_account(&user_share_ata).unwrap();
    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert!(user_share_balance_after > 0);

    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_amount);
}

#[test]
fn test_deposit_with_hook_two_protocols() {
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

    init_deposit_hook(
        &mut svm,
        &authority,
        &share_mint.pubkey(),
        &vault_pubkey,
        HOOK_PROGRAM_ID,
    )
    .expect("init deposit hook failed");

    init_deposit_extra_meta_accounts(
        &mut svm,
        &authority,
        &asset_mint.pubkey(),
        &share_mint.pubkey(),
        &vault_pubkey,
    )
    .expect("init deposit extra meta accounts failed");

    init_vault(&mut svm, &authority, &share_mint.pubkey(), &vault_pubkey)
        .expect("init vault failed");

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

    let (vault_associated_protocols_pubkey, _) = Pubkey::find_program_address(
        &[b"vault_associated_protocols", vault_pubkey.as_ref()],
        &hook_program_id(),
    );

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

    let add_first_protocol_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        AddAssociatedProtocolBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .protocol(system_program::ID)
            .instruction(),
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[add_first_protocol_ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );
    svm.send_transaction(tx)
        .expect("add first associated protocol failed");

    // Add dummy program as the SECOND associated protocol
    let add_second_protocol_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        AddAssociatedProtocolBuilder::new()
            .authority(authority.pubkey())
            .vault(vault_pubkey)
            .vault_associated_protocols(vault_associated_protocols_pubkey)
            .protocol(dummy_program_id())
            .instruction(),
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[add_second_protocol_ix],
        Some(&authority.pubkey()),
        &[&authority],
        blockhash,
    );
    svm.send_transaction(tx)
        .expect("add second associated protocol failed");

    // Set up user accounts
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

    let (extra_meta_pubkey, _) = Pubkey::find_program_address(
        &[
            b"extra_account_metas",
            b"deposit",
            share_mint.pubkey().as_ref(),
        ],
        &hook_program_id(),
    );

    let (nav_return_data_pubkey, _) = Pubkey::find_program_address(
        &[b"vault_nav_data", vault_pubkey.as_ref()],
        &hook_program_id(),
    );

    let external_nav_amount: u64 = 100_000;

    let protocol_deposits_discriminator: [u8; 8] = {
        let hash = solana_sdk::hash::hashv(&[b"account:ProtocolDeposits"]);
        hash.to_bytes()[..8].try_into().unwrap()
    };

    let (protocol_deposits_pubkey, protocol_deposits_bump) = Pubkey::find_program_address(
        &[
            b"vault_protocol_deposit",
            vault_pubkey.as_ref(),
            dummy_program_id().as_ref(),
        ],
        &hook_program_id(),
    );

    let mut protocol_deposits_data = Vec::with_capacity(81);
    protocol_deposits_data.extend_from_slice(&protocol_deposits_discriminator);
    protocol_deposits_data.extend_from_slice(vault_pubkey.as_ref());
    protocol_deposits_data.extend_from_slice(dummy_program_id().as_ref());
    protocol_deposits_data.extend_from_slice(&external_nav_amount.to_le_bytes());
    protocol_deposits_data.push(protocol_deposits_bump);

    svm.set_account(
        protocol_deposits_pubkey,
        solana_sdk::account::Account {
            lamports: svm.minimum_balance_for_rent_exemption(protocol_deposits_data.len()),
            data: protocol_deposits_data,
            owner: hook_program_id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("set protocol deposits account failed");

    let mut update_nav_ix = hook_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        UpdateNavBuilder::new()
            .payer(user.pubkey())
            .vault(vault_pubkey)
            .associated_protocols_info(vault_associated_protocols_pubkey)
            .nav_return_data(nav_return_data_pubkey)
            .instruction(),
    );
    update_nav_ix
        .accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            protocol_deposits_pubkey,
            false,
        ));

    let deposit_amount = 500_000;
    let mut ix = vault_client::sdk::IntoSdkInstruction::into_sdk_instruction(
        DepositBuilder::new()
            .user(user.pubkey())
            .asset_mint(asset_mint.pubkey())
            .share_mint(share_mint.pubkey())
            .reserve(reserve_pubkey)
            .vault(vault_pubkey)
            .fee_recipient(fee_recipient_ata)
            .extra_metas(Some(extra_meta_pubkey))
            .user_assets_account(user_asset_ata)
            .user_shares_account(user_share_ata)
            .asset_token_program(token::ID)
            .share_token_program(token::ID)
            .hook_program(Some(HOOK_PROGRAM_ID))
            .protocol(Some(dummy_program_id()))
            .nav_return_data(Some(nav_return_data_pubkey))
            .instructions(Some(solana_sdk::sysvar::instructions::ID))
            .assets(deposit_amount)
            .min_shares(0)
            .instruction(),
    );

    ix.accounts.push(solana_sdk::instruction::AccountMeta::new(
        dummy_vault_pubkey,
        false,
    ));

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[update_nav_ix, ix],
        Some(&user.pubkey()),
        &[&user],
        blockhash,
    );
    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "deposit into second protocol failed: {:?}",
        result.err()
    );

    // Verify user asset balance decreased
    let user_asset_ata_account = svm.get_account(&user_asset_ata).unwrap();
    let user_asset_balance_after = TokenAccount::unpack(user_asset_ata_account.data())
        .unwrap()
        .amount;
    assert_eq!(
        user_asset_balance_after,
        user_asset_amount.checked_sub(deposit_amount).unwrap()
    );

    // Verify user received shares
    let user_share_ata_account = svm.get_account(&user_share_ata).unwrap();
    let user_share_balance_after = TokenAccount::unpack(user_share_ata_account.data())
        .unwrap()
        .amount;
    assert!(user_share_balance_after > 0);

    // Verify reserve received assets
    let vault_asset_balance = get_vault_asset_balance(&svm, &vault_pubkey);
    assert_eq!(vault_asset_balance, deposit_amount);

    // Verify the NAV reflects the seeded ProtocolDeposits amount.
    // update_nav read the ProtocolDeposits PDA for the second protocol and stored it in
    // nav_return_data; the deposit then priced shares against that non-zero NAV.
    let nav_account = svm
        .get_account(&nav_return_data_pubkey)
        .expect("nav_return_data account should exist");
    let nav_data =
        NavReturnData::from_bytes(nav_account.data()).expect("nav deserialization failed");
    assert_eq!(nav_data.nav, external_nav_amount);
}

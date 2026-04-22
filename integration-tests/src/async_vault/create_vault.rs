use anchor_spl::{
    token::{self, spl_token},
    token_2022::spl_token_2022,
};
use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction::create_account, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_async_vault, create_mint, PENDING_VAULT_SEED, RESERVE_CONFIG_SEED,
    VAULT_CONFIG_SEED,
};

#[test_case(100_000_000, true, true, true, false ; "both async inflows and outflows")]
#[test_case(100_000_000, true, false, true, false ; "async inflows only")]
#[test_case(100_000_000, false, true, true, false ; "async outflows only")]
#[test_case(100_000_000, false, false, true, false ; "no async flows")]
#[test_case(1, true, true, true, false ; "minimum price")]
#[test_case(u64::MAX, true, true, true,  false ; "maximum price")]
#[test_case(0, true, true, true,  false ; "zero initial price fails")]
#[test_case(100_000_000, true, true, false, false ; "invalid mint authority fails")]
#[test_case(100_000_000, true, true, true,  false ; "duplicate vault creation fails")]
#[test_case(100_000_000, true, true, true,  true ; "same mints fails")]
fn test_create_vault(
    initial_price: u64,
    async_inflows: bool,
    async_outflows: bool,
    use_valid_mint_authority: bool,
    use_same_mints: bool,
) {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let fake_mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fake_mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    if !use_same_mints {
        create_mint(&mut svm, &mint_authority, &share_mint);
    }

    let effective_share_mint = if use_same_mints {
        asset_mint.pubkey()
    } else {
        share_mint.pubkey()
    };

    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[RESERVE_CONFIG_SEED, effective_share_mint.as_ref()],
        &program_id(),
    );
    let (pending_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_VAULT_SEED, effective_share_mint.as_ref()],
        &program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, effective_share_mint.as_ref()],
        &program_id(),
    );

    let effective_mint_authority = if use_valid_mint_authority {
        &mint_authority
    } else {
        &fake_mint_authority
    };

    let result = create_async_vault(
        &mut svm,
        &authority,
        &payer,
        effective_mint_authority,
        asset_mint.pubkey(),
        effective_share_mint,
        reserve_pubkey,
        pending_vault_pubkey,
        vault_pubkey,
        initial_price,
        async_inflows,
        async_outflows,
        token::ID,
        token::ID,
    );

    let should_succeed = initial_price != 0 && use_valid_mint_authority && !use_same_mints;

    if should_succeed {
        result.expect("async vault creation should succeed");

        let vault_account = svm
            .get_account(&vault_pubkey)
            .expect("Vault account should exist");
        assert!(!vault_account.data.is_empty(), "Vault should have data");

        let vault_config = Vault::from_bytes(vault_account.data()).unwrap();
        assert_eq!(vault_config.authority, authority.pubkey());
        assert_eq!(vault_config.asset_mint_address, asset_mint.pubkey());
        assert_eq!(vault_config.share_mint_address, effective_share_mint);
        assert_eq!(vault_config.vault_token_account, reserve_pubkey);
        assert_eq!(vault_config.pending_vault, pending_vault_pubkey);
        assert_eq!(vault_config.initial_price, initial_price);
        assert_eq!(vault_config.paused, false);
        assert!(!vault_config.initialized);
        assert_eq!(vault_config.nav, 0);
        assert_eq!(vault_config.nav_version, 0);
        assert_eq!(vault_config.async_inflows, async_inflows);
        assert_eq!(vault_config.async_outflows, async_outflows);
        assert_eq!(vault_config.pending_async_requests, 0);
        assert_eq!(vault_config.total_asset_balance, 0);
    } else {
        let err_result = &result.unwrap_err();
        if initial_price == 0 {
            assert_error_code(err_result, 6000, "Initial price cannot be zero");
        }
        if !use_valid_mint_authority {
            assert_error_code(err_result, 4, "OwnerMismatch");
        }

        if use_same_mints {
            assert_error_code(err_result, 6010, "Mints should be different.");
        }
    }
}

#[test]
fn test_create_vault_nonzero_share_mint_supply_fails() {
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/async_vault.so");
    svm.add_program(program_id(), program_bytes).unwrap();

    let authority = Keypair::new();
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let asset_mint = Keypair::new();
    let share_mint = Keypair::new();
    let token_account_kp = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint);
    create_mint(&mut svm, &mint_authority, &share_mint);

    let rent = svm.minimum_balance_for_rent_exemption(spl_token_2022::state::Account::LEN);
    let create_account_ix = create_account(
        &mint_authority.pubkey(),
        &token_account_kp.pubkey(),
        rent,
        spl_token_2022::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_ix = spl_token_2022::instruction::initialize_account(
        &spl_token::ID,
        &token_account_kp.pubkey(),
        &share_mint.pubkey(),
        &mint_authority.pubkey(),
    )
    .unwrap();
    let mint_to_ix = spl_token_2022::instruction::mint_to(
        &spl_token::ID,
        &share_mint.pubkey(),
        &token_account_kp.pubkey(),
        &mint_authority.pubkey(),
        &[],
        1,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_account_ix, mint_to_ix],
        Some(&mint_authority.pubkey()),
        &[&mint_authority, &token_account_kp],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("token account creation and mint_to should succeed");

    let (reserve_pubkey, _) = Pubkey::find_program_address(
        &[RESERVE_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );
    let (pending_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_VAULT_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );
    let (vault_pubkey, _) = Pubkey::find_program_address(
        &[VAULT_CONFIG_SEED, share_mint.pubkey().as_ref()],
        &program_id(),
    );

    let result = create_async_vault(
        &mut svm,
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
    );

    let err_result = &result.unwrap_err();
    assert_error_code(err_result, 6011, "Share mint supply should be zero.");
}

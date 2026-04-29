use anchor_spl::{
    token::{self, spl_token},
    token_2022::{self, spl_token_2022},
};
use async_vault_client::{sdk::program_id, Vault};
use litesvm::LiteSVM;
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction::create_account, transaction::Transaction,
};
use test_case::test_case;

use crate::helper_functions::{
    assert_error_code, create_async_vault, create_mint, PENDING_SHARES_VAULT_SEED,
    PENDING_VAULT_SEED, RESERVE_CONFIG_SEED, VAULT_CONFIG_SEED,
};

#[test_case(100_000_000, true, true, true, false, token::ID,token::ID ; "both async inflows and outflows")]
#[test_case(100_000_000, true, true, true, false, token_2022::ID,token_2022::ID ; "Token 2022 program for both mints")]
#[test_case(100_000_000, true, true, true, false, token::ID,token_2022::ID ; "Token program for asset, Token program 2022 for share")]
#[test_case(100_000_000, true, true, true, false, token::ID,token_2022::ID ; "Token 2022 program for asset, Token program for share")]
#[test_case(100_000_000, true, false, true, false,token_2022::ID,token_2022::ID ; "async inflows only")]
#[test_case(100_000_000, false, true, true, false,token_2022::ID,token_2022::ID ; "async outflows only")]
#[test_case(100_000_000, false, false, true, false,token_2022::ID,token_2022::ID ; "no async flows")]
#[test_case(1, true, true, true, false,token_2022::ID,token_2022::ID ; "minimum price")]
#[test_case(u64::MAX, true, true, true,  false,token_2022::ID,token_2022::ID ; "maximum price")]
#[test_case(0, true, true, true,  false,token_2022::ID,token_2022::ID ; "zero initial price fails")]
#[test_case(100_000_000, true, true, false, false,token_2022::ID,token_2022::ID ; "invalid mint authority fails")]
#[test_case(100_000_000, true, true, true,  false,token_2022::ID,token_2022::ID ; "duplicate vault creation fails")]
#[test_case(100_000_000, true, true, true,  true, token_2022::ID,token_2022::ID; "same mints fails")]
fn test_create_vault(
    initial_price: u64,
    async_inflows: bool,
    async_outflows: bool,
    use_valid_mint_authority: bool,
    use_same_mints: bool,
    asset_program: Pubkey,
    share_program: Pubkey,
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
    let fee_recipient = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&fake_mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint, &asset_program);
    if !use_same_mints {
        create_mint(&mut svm, &mint_authority, &share_mint, &share_program);
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
    let (pending_shares_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_SHARES_VAULT_SEED, effective_share_mint.as_ref()],
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
        fee_recipient.pubkey(),
        asset_mint.pubkey(),
        effective_share_mint,
        reserve_pubkey,
        pending_vault_pubkey,
        pending_shares_vault_pubkey,
        vault_pubkey,
        initial_price,
        async_inflows,
        async_outflows,
        asset_program,
        share_program,
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
    let fee_recipient = Keypair::new();

    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&fee_recipient.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    create_mint(&mut svm, &mint_authority, &asset_mint, &spl_token::ID);
    create_mint(&mut svm, &mint_authority, &share_mint, &spl_token::ID);

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
    let (pending_shares_vault_pubkey, _) = Pubkey::find_program_address(
        &[PENDING_SHARES_VAULT_SEED, share_mint.pubkey().as_ref()],
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
        fee_recipient.pubkey(),
        asset_mint.pubkey(),
        share_mint.pubkey(),
        reserve_pubkey,
        pending_vault_pubkey,
        pending_shares_vault_pubkey,
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

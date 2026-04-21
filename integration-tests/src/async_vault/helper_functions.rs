use anchor_spl::{token::spl_token, token_2022::spl_token_2022};
use async_vault_client::{sdk::IntoSdkInstruction as _, InitializeVaultBuilder};
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction::create_account,
    system_program,
    transaction::Transaction,
};

pub fn async_vault_program_id() -> Pubkey {
    "2kUpRoU8oGpstygkk3ZE51upGSq9UpkjNoEUiiQ88MMY"
        .parse()
        .unwrap()
}

pub fn create_mint(svm: &mut LiteSVM, signer: &Keypair, mint: &Keypair) {
    use spl_token_2022::state::Mint;
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);
    let init_account_ix = create_account(
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
        &[mint, signer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(init_tx)
        .expect("create_mint transaction failed");
}

pub fn create_async_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    payer: &Keypair,
    mint_authority: &Keypair,
    asset_mint: Pubkey,
    share_mint: Pubkey,
    reserve: Pubkey,
    pending_vault: Pubkey,
    vault: Pubkey,
    initial_price: u64,
    async_inflows: bool,
    async_outflows: bool,
    asset_token_program: Pubkey,
    share_token_program: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let disc = {
        let hash = solana_sdk::hash::hash(b"global:create_vault");
        let b = hash.to_bytes();
        [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
    };

    let mut data = disc.to_vec();
    data.extend_from_slice(authority.pubkey().as_ref());
    data.extend_from_slice(&initial_price.to_le_bytes());
    data.push(async_inflows as u8);
    data.push(async_outflows as u8);

    let ix = Instruction {
        program_id: async_vault_program_id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(mint_authority.pubkey(), true),
            AccountMeta::new_readonly(asset_mint, false),
            AccountMeta::new(share_mint, false),
            AccountMeta::new(reserve, false),
            AccountMeta::new(pending_vault, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(asset_token_program, false),
            AccountMeta::new_readonly(share_token_program, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        blockhash,
    );

    svm.send_transaction(tx)
}

pub struct AsyncVaultAccount {
    pub asset_mint_address: Pubkey,
    pub share_mint_address: Pubkey,
    pub vault_token_account: Pubkey,
    pub authority: Pubkey,
    pub initial_price: u64,
    pub paused: bool,
    pub initialized: bool,
    pub pending_vault: Pubkey,
    pub nav: u128,
    pub nav_version: u64,
    pub async_inflows: bool,
    pub async_outflows: bool,
    pub pending_async_requests: u16,
    pub total_asset_balance: u64,
    pub reserve_bump: u8,
    pub pending_vault_bump: u8,
    pub bump: u8,
}

pub fn initialize_async_vault(
    svm: &mut LiteSVM,
    authority: &Keypair,
    share_mint: Pubkey,
    vault: Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let ix = InitializeVaultBuilder::new()
        .authority(async_vault_client::Pubkey::new_from_array(
            authority.pubkey().to_bytes(),
        ))
        .share_mint(async_vault_client::Pubkey::new_from_array(
            share_mint.to_bytes(),
        ))
        .vault(async_vault_client::Pubkey::new_from_array(vault.to_bytes()))
        .instruction()
        .into_sdk_instruction();

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        blockhash,
    );

    svm.send_transaction(tx)
}

impl AsyncVaultAccount {
    pub fn from_account_data(data: &[u8]) -> Self {
        let mut o = 8; // skip 8-byte Anchor discriminator

        let read_pubkey = |d: &[u8], o: &mut usize| -> Pubkey {
            let arr: [u8; 32] = d[*o..*o + 32].try_into().unwrap();
            *o += 32;
            Pubkey::new_from_array(arr)
        };
        let read_u64 = |d: &[u8], o: &mut usize| -> u64 {
            let arr: [u8; 8] = d[*o..*o + 8].try_into().unwrap();
            *o += 8;
            u64::from_le_bytes(arr)
        };
        let read_u128 = |d: &[u8], o: &mut usize| -> u128 {
            let arr: [u8; 16] = d[*o..*o + 16].try_into().unwrap();
            *o += 16;
            u128::from_le_bytes(arr)
        };
        let read_u16 = |d: &[u8], o: &mut usize| -> u16 {
            let arr: [u8; 2] = d[*o..*o + 2].try_into().unwrap();
            *o += 2;
            u16::from_le_bytes(arr)
        };
        let read_bool = |d: &[u8], o: &mut usize| -> bool {
            let v = d[*o] != 0;
            *o += 1;
            v
        };
        let read_u8 = |d: &[u8], o: &mut usize| -> u8 {
            let v = d[*o];
            *o += 1;
            v
        };

        Self {
            asset_mint_address: read_pubkey(data, &mut o),
            share_mint_address: read_pubkey(data, &mut o),
            vault_token_account: read_pubkey(data, &mut o),
            authority: read_pubkey(data, &mut o),
            initial_price: read_u64(data, &mut o),
            paused: read_bool(data, &mut o),
            initialized: read_bool(data, &mut o),
            pending_vault: read_pubkey(data, &mut o),
            nav: read_u128(data, &mut o),
            nav_version: read_u64(data, &mut o),
            async_inflows: read_bool(data, &mut o),
            async_outflows: read_bool(data, &mut o),
            pending_async_requests: read_u16(data, &mut o),
            total_asset_balance: read_u64(data, &mut o),
            reserve_bump: read_u8(data, &mut o),
            pending_vault_bump: read_u8(data, &mut o),
            bump: read_u8(data, &mut o),
        }
    }
}

use anchor_lang::pubkey;

pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra_account_metas";
pub const DEPOSIT_ACCOUNT_METAS_SEED: &[u8] = b"deposit";
pub const VAULT_ASSOCIATED_PROTOCOLS_SEED: &[u8] = b"vault_associated_protocols";
pub const VAULT_PROTOCOL_DEPOSIT_SEED: &[u8] = b"vault_protocol_deposit";
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_NAV_DATA_SEED: &[u8] = b"vault_nav_data";
pub const VAULT_PROGRAM_ID: &anchor_lang::prelude::Pubkey =
    &pubkey!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw");

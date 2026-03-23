use anchor_lang::pubkey;

pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra_account_metas";
pub const DEPOSIT_ACCOUNT_METAS_SEED: &[u8] = b"deposit";
pub const WITHDRAW_ACCOUNT_METAS_SEED: &[u8] = b"withdraw";
pub const VAULT_ASSOCIATED_PROTOCOLS: &[u8] = b"vault_associated_protocols";
pub const VAULT_PROTOCOL_DEPOSIT: &[u8] = b"vault_protocol_deposit";
pub const VAULT_NAV_DATA: &[u8] = b"vault_nav_data";
pub const UPDATE_NAV_DISCRIMINATOR: [u8; 8] = [56, 16, 234, 109, 155, 165, 5, 0];
pub const VAULT_PROGRAM_ID: &anchor_lang::prelude::Pubkey =
    &pubkey!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw");

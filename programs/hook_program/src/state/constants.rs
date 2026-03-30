use anchor_lang::pubkey;

use sha2_const_stable::Sha256;

pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra_account_metas";
pub const DEPOSIT_ACCOUNT_METAS_SEED: &[u8] = b"deposit";
pub const WITHDRAW_ACCOUNT_METAS_SEED: &[u8] = b"withdraw";
pub const VAULT_ASSOCIATED_PROTOCOLS_SEED: &[u8] = b"vault_associated_protocols";
pub const VAULT_PROTOCOL_DEPOSIT_SEED: &[u8] = b"vault_protocol_deposit";
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_NAV_DATA_SEED: &[u8] = b"vault_nav_data";

pub const VAULT_PROGRAM_ID: &anchor_lang::prelude::Pubkey =
    &pubkey!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw");

pub const fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let hash = Sha256::new()
        .update(namespace.as_bytes())
        .update(b":")
        .update(name.as_bytes())
        .finalize();

    [
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ]
}

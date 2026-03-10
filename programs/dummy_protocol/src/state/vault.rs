use anchor_lang::prelude::*;

pub const VAULT_CONFIG_SEED: &[u8] = b"vault";

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub amount_deposit: u64,
    pub bump: u8,
}

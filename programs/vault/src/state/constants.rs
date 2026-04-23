pub const VAULT_CONFIG_SEED: &[u8] = b"vault";
pub const RESERVE_CONFIG_SEED: &[u8] = b"reserve";
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra_account_metas";
pub const DEPOSIT_ACCOUNT_METAS_SEED: &[u8] = b"deposit";
pub const WITHDRAW_ACCOUNT_METAS_SEED: &[u8] = b"withdraw";

pub enum Rounding {
    /// Rounding up
    Up,
    /// Rounding down
    Down,
}

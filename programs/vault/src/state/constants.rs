pub const VAULT_CONFIG_SEED: &[u8] = b"vault";
pub const RESERVE_CONFIG_SEED: &[u8] = b"reserve";
pub const MAX_BPS: u16 = 10_000;
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra_account_metas";
pub const DEPOSIT_ACCOUNT_METAS_SEED: &[u8] = b"deposit";
pub enum Rounding {
    /// Rounding up
    Up,
    /// Rounding down
    Down,
}

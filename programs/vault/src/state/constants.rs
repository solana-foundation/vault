pub const VAULT_CONFIG_SEED: &[u8] = b"vault";
pub const RESERVE_CONFIG_SEED: &[u8] = b"reserve";

pub enum Rounding {
    /// Rounding up
    Up,
    /// Rounding down
    Down,
}

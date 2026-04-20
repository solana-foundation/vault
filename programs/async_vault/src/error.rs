use anchor_lang::prelude::*;

#[error_code]
pub enum AsyncVaultError {
    #[msg("Initial price cannot be zero")]
    InvalidInitialPrice,
}

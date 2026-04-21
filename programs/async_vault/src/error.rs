use anchor_lang::prelude::*;

#[error_code]
pub enum AsyncVaultError {
    #[msg("Initial price cannot be zero")]
    InvalidInitialPrice,
    #[msg("Signer is not the vault authority")]
    UnauthorizedSigner,
    #[msg("Vault is already initialized")]
    VaultAlreadyInitialized,
}

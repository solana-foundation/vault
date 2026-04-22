use anchor_lang::prelude::*;

#[error_code]
pub enum AsyncVaultError {
    #[msg("Initial price cannot be zero")]
    InvalidInitialPrice,
    #[msg("Unauthorized signer")]
    UnauthorizedSigner,
    #[msg("Vault is not initialized")]
    UninitializedVault,
    #[msg("Vault is paused")]
    PausedVault,
    #[msg("Vault is already initialized")]
    VaultAlreadyInitialized,
    #[msg("Extension is already initialized")]
    ExtensionAlreadyInitialized,
    #[msg("Extension is not initialized")]
    UninitializedExtension,
    #[msg("Invalid extension data")]
    InvalidExtensionData,
    #[msg("Fee basis points exceed maximum")]
    FeeBpsExceeded,
    #[msg("Arithmetic error")]
    ArithmeticError,
    #[msg("Mints should be different.")]
    MintsShouldBeDifferent,
}

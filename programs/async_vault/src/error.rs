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
    #[msg("Share mint supply should be zero.")]
    ShareMintSupplyShouldBeZero,
    #[msg("No pending authority invitation")]
    NoPendingAuthority,
    #[msg("Pending Vault is not valid.")]
    InvalidPendingVault,
    #[msg("Fee recipient account must be provided as a remaining account when fee > 0.")]
    MissingFeeRecipient,
    #[msg("Fee recipient account is invalid.")]
    InvalidFeeRecipient,
}

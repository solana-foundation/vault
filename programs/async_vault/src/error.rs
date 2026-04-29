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
    #[msg("Pending Shares Vault is not valid.")]
    InvalidPendingSharesVault,
    #[msg("Fee recipient account must be provided as a remaining account when fee > 0.")]
    MissingFeeRecipient,
    #[msg("Fee recipient account is invalid.")]
    InvalidFeeRecipient,
    #[msg("Request is not pending.")]
    RequestIsNotPending,
    #[msg("Invalid request type for this instruction.")]
    InvalidRequestType,
    #[msg("A required optional account was not provided.")]
    MissingRequiredAccount,
    #[msg("Asset mint has invalid extensions.")]
    InvalidAssetMintExtensions,
    #[msg("Asset mint is not valid.")]
    InvalidAssetMint,
    #[msg("Share mint is not valid.")]
    InvalidShareMint,
    #[msg("Request address is not valid.")]
    InvalidRequest,
    #[msg("Request is not in a Pending state.")]
    RequestNotPending,
}

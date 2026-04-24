use anchor_lang::prelude::*;

#[error_code]
pub enum VaultProgramError {
    #[msg("The provided fee must not exceed 100% (10,000 bps).")]
    FeeBPSLimitReached,

    #[msg("The provided signer is not allowed to execute this instruction.")]
    UnauthorizedSigner,

    #[msg("Something happened while performing an arithmetic operation.")]
    ArithmeticError,

    #[msg("The vault is paused.")]
    PausedVault,

    #[msg("The vault max asset cap has been exceeded.")]
    MaxVaultAssetCapExceeded,

    #[msg("The provided mint supply should be zero.")]
    MintSupplyShouldBeZero,

    #[msg("The provided share supply should be zero.")]
    ShareSupplyShouldBeZero,

    #[msg("The provided vault reserve should be empty in order to close it.")]
    VaultShouldBeEmpty,

    #[msg("Deposit amount too small to mint shares.")]
    InsufficientDepositAmount,

    #[msg("Initial price has to be bigger than 0")]
    InvalidInitialPrice,

    #[msg("Withdraw amount too small to burn shares.")]
    InsufficientWithdrawAmount,

    #[msg("Redeem shares amount too small.")]
    InsufficientRedeemAmount,

    #[msg("Invalid vault state for this operation.")]
    InvalidState,

    #[msg("Slippage exceeded.")]
    SlippageExceeded,

    #[msg("Vault is already initialized.")]
    VaultAlreadyInitialized,

    #[msg("The extension is already initialized.")]
    ExtensionAlreadyInitialized,

    #[msg("The vault is not initialized.")]
    UninitializedVault,

    #[msg("The extension is not initialized.")]
    UninitializedExtension,

    #[msg("The provided vault and mint mismatch.")]
    VaultShareMintMismatch,

    #[msg("The vault NAV is stale. Please update the NAV value before depositing.")]
    StaleVaultNav,

    #[msg("This instruction needs the hook extension to be initialized.")]
    HookExtensionNotInitialized,

    #[msg("The provided optional account is empty.")]
    OptionalAccountIsEmpty,

    #[msg("The returned data is invalid.")]
    InvalidReturnedData,

    #[msg("The provided extra meta accounts pubkey does not match")]
    InvalidAccountData,

    #[msg("This instruction is not available when a hook extension is active. Use the hook-aware instruction instead.")]
    HookExtensionActive,

    #[msg("Mints should be different.")]
    MintsShouldBeDifferent,

    #[msg("Share mint supply should be zero.")]
    ShareMintSupplyShouldBeZero,

    #[msg("Async inflows are disabled, cannot deposit.")]
    AsyncInflowsDisabled,

    #[msg("Nav is not set.")]
    NavIsNotSet,
}

use thiserror::Error;

/// Pure math/validation errors produced by [`crate::fee`].
///
/// `vault_common` deliberately carries no `#[error_code]` enum: Anchor 1.0 allows
/// only one per program. The program crate maps these into its single
/// `AsyncVaultError` at the call boundary.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultMathError {
    #[error("Something happened while performing an arithmetic operation.")]
    ArithmeticError,

    #[error("The provided fee must not exceed 100% (10,000 bps).")]
    FeeBpsLimitReached,
}

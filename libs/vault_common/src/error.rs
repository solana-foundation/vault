use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultMathError {
    #[error("Something happened while performing an arithmetic operation.")]
    ArithmeticError,

    #[error("The provided fee must not exceed 100% (10,000 bps).")]
    FeeBpsLimitReached,
}

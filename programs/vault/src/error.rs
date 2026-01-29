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
}

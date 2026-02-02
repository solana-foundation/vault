use anchor_lang::prelude::*;

#[error_code]
pub enum VaultProgramError {
    #[msg("The provided fee must not exceed 100% (10,000 bps).")]
    FeeBPSLimitReached,

    #[msg("The provided signer is not allowed to execute this instruction.")]
    UnauthorizedSigner,

    #[msg("The provided mint supply should be zero.")]
    MintSupplyShouldBeZero,

    #[msg("The provided vault reserve should be empty in order to close it.")]
    VaultShouldBeEmpty,
}

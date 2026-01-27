use anchor_lang::prelude::*;

#[error_code]
pub enum VaultProgramError {
    #[msg("The provided fee must not exceed 100% (10,000 bps).")]
    FeeBPSLimitReached,
}

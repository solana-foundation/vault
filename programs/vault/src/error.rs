use anchor_lang::prelude::*;

#[error_code]
pub enum VaultProgramError {
    #[msg("The provided fee should not be 100 percent.")]
    FeeBPSLimitReached,
}

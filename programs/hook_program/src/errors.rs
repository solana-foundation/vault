use anchor_lang::prelude::*;

#[error_code]
pub enum HookProgramError {
    #[msg("Protocol already associated with this vault")]
    ProtocolAlreadyAssociated,
    #[msg("Protocol not found in vault associations")]
    ProtocolNotFound,
    #[msg("Maximum number of associated protocols reached")]
    MaxProtocolsReached,
    #[msg("update_nav must be called before get_nav in the same transaction")]
    UpdateNavNotCalledBeforeGetNav,
    #[msg("The provided extra meta accounts pubkey does not match")]
    InvalidAccountData,
    #[msg("Signer is not the vault authority")]
    UnauthorizedAuthority,
    #[msg("At least 2 associated protocols are required")]
    InsufficientAssociatedProtocols,
    #[msg("Something happened while performing an arithmetic operation.")]
    ArithmeticError,
}

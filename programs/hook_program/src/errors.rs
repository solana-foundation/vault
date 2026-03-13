use anchor_lang::prelude::*;

#[error_code]
pub enum HookProgramError {
    #[msg("Protocol already associated with this vault")]
    ProtocolAlreadyAssociated,
    #[msg("Protocol not found in vault associations")]
    ProtocolNotFound,
    #[msg("Maximum number of associated protocols reached")]
    MaxProtocolsReached,
}

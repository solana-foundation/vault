use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub struct DepositHook {
    pub hook_program_id: Pubkey,
    pub authority: Pubkey,
}

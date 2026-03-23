use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, InitSpace, Copy)]
pub struct WithdrawHook {
    pub hook_program_id: Pubkey,
    pub authority: Pubkey,
}

use anchor_lang::prelude::*;
pub mod instructions;
pub mod state;

declare_id!("4QabXWDFDL3cVzpabsVNCjkjgHvMAfTwPy6kCV9HiB7n");

use crate::state::DepositHookInstruction;
use instructions::*;
use spl_discriminator::SplDiscriminate;

#[program]
pub mod hook_program {

    use super::*;

    #[instruction(discriminator = DepositHookInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn execute_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositHook<'info>>,
    ) -> Result<()> {
        instructions::deposit_hook::handler(ctx)
    }
}

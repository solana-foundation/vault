use anchor_lang::prelude::*;
pub mod instructions;

declare_id!("4QabXWDFDL3cVzpabsVNCjkjgHvMAfTwPy6kCV9HiB7n");

use instructions::*;

#[program]
pub mod hook_program {
    use super::*;

    pub fn execute_deposit(ctx: Context<DepositHook>) -> Result<()> {
        instructions::deposit_hook::handler(ctx)
    }
}

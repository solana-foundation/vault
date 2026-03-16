use anchor_lang::prelude::*;
pub mod errors;
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

    pub fn get_nav<'info>(ctx: Context<GetNavData>) -> Result<()> {
        instructions::get_nav_data::handler(ctx)
    }

    pub fn update_nav<'info>(
        ctx: Context<'_, '_, 'info, 'info, UpdateNavData<'info>>,
    ) -> Result<()> {
        instructions::update_nav_data::handler(ctx)
    }

    pub fn init_vault_associated_protocols(
        ctx: Context<InitVaultAssociatedProtocols>,
    ) -> Result<()> {
        instructions::init_vault_associated_protocols::handler(ctx)
    }

    pub fn add_associated_protocol(ctx: Context<AddAssociatedProtocol>) -> Result<()> {
        instructions::add_associated_protocol::handler(ctx)
    }

    pub fn remove_associated_protocol(ctx: Context<RemoveAssociatedProtocol>) -> Result<()> {
        instructions::remove_associated_protocol::handler(ctx)
    }
}

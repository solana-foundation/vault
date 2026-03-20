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

    /// Transfer hook entrypoint called by the vault program deposit instruction.
    /// Resolves extra account metas from the validation PDA and invokes the downstream
    /// protocol's deposit hook via CPI using the `DepositHookInstruction` discriminator.
    #[instruction(discriminator = DepositHookInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn execute_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteDepositHook<'info>>,
    ) -> Result<()> {
        instructions::execute_deposit_hook::handler(ctx)
    }

    /// Returns the vault's current Net Asset Value (NAV) as transaction return data.
    /// Reads the stored `NavReturnData` PDA and sets it as the return data for the caller.
    /// Requires that `update_nav` was called earlier in the same transaction for the same vault.
    pub fn get_nav<'info>(ctx: Context<GetNavData>) -> Result<()> {
        instructions::get_nav_data::handler(ctx)
    }

    /// Aggregates deposit amounts across all associated protocols and stores the total NAV.
    /// Iterates through each protocol in `VaultAssociatedProtocols`, reads its `ProtocolDeposits`
    /// PDA from `remaining_accounts`, and sums the amounts into the `NavReturnData` PDA.
    /// Creates the `NavReturnData` account if it does not yet exist.
    pub fn update_nav<'info>(
        ctx: Context<'_, '_, 'info, 'info, UpdateNavData<'info>>,
    ) -> Result<()> {
        instructions::update_nav_data::handler(ctx)
    }

    /// Initializes the `VaultAssociatedProtocols` PDA for a given vault.
    /// Creates an empty protocol list and records the vault pubkey and bump seed.
    /// Must be called before any protocols can be added or NAV can be computed.
    pub fn init_vault_associated_protocols(
        ctx: Context<InitVaultAssociatedProtocols>,
    ) -> Result<()> {
        instructions::init_vault_associated_protocols::handler(ctx)
    }

    /// Adds a protocol pubkey to the vault's associated protocols list.
    /// Errors if the protocol is already associated or if the list has reached the maximum
    /// of 10 protocols.
    pub fn add_associated_protocol(ctx: Context<AddAssociatedProtocol>) -> Result<()> {
        instructions::add_associated_protocol::handler(ctx)
    }

    /// Removes a protocol pubkey from the vault's associated protocols list.
    /// Errors if the specified protocol is not currently associated with the vault.
    pub fn remove_associated_protocol(ctx: Context<RemoveAssociatedProtocol>) -> Result<()> {
        instructions::remove_associated_protocol::handler(ctx)
    }

    // Extra Meta Accounts

    /// Initializes the deposit hook extra meta accounts needed for the deposit hook
    pub fn initialize_deposit_extra_meta_accounts(
        ctx: Context<InitializeDepositExtraMetaAccounts>,
    ) -> Result<()> {
        instructions::initialize_deposit_extra_meta_accounts::handler(ctx)
    }
}

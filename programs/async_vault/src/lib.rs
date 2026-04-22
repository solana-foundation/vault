use anchor_lang::prelude::*;

pub mod error;
pub mod extensions;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("2kUpRoU8oGpstygkk3ZE51upGSq9UpkjNoEUiiQ88MMY");

#[program]
pub mod async_vault {
    use super::*;

    /// Creates a new async vault with reserve and pending token accounts,
    /// transfers share mint authority to the vault PDA, and initializes
    /// the vault config in a paused + uninitialized state.
    pub fn create_vault(ctx: Context<CreateVault>, args: AsyncVaultArgs) -> Result<()> {
        instructions::create_vault::handler(ctx, args)
    }

    pub fn initialize_deposit_fee(
        ctx: Context<InitDepositFee>,
        args: InitDepositFeeArgs,
    ) -> Result<()> {
        instructions::initialize_deposit_fee::handler(ctx, args)
    }

    pub fn initialize_withdrawal_fee(
        ctx: Context<InitWithdrawalFee>,
        args: InitWithdrawalFeeArgs,
    ) -> Result<()> {
        instructions::initialize_withdrawal_fee::handler(ctx, args)
    }

    pub fn update_deposit_fee(
        ctx: Context<UpdateDepositFee>,
        args: UpdateDepositFeeArgs,
    ) -> Result<()> {
        instructions::update_deposit_fee::handler(ctx, args)
    }

    pub fn update_withdrawal_fee(
        ctx: Context<UpdateWithdrawalFee>,
        args: UpdateWithdrawalFeeArgs,
    ) -> Result<()> {
        instructions::update_withdrawal_fee::handler(ctx, args)
    }
}

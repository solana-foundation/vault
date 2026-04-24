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

    /// Finalizes vault setup by marking it as initialized. Once initialized,
    /// no new extensions can be added. Requires authority signature.
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault::handler(ctx)
    }

    /// Adds a deposit fee TLV extension to the vault. Must be called
    /// before vault initialization. Requires authority signature.
    pub fn initialize_deposit_fee(
        ctx: Context<InitDepositFee>,
        args: InitDepositFeeArgs,
    ) -> Result<()> {
        instructions::initialize_deposit_fee::handler(ctx, args)
    }

    /// Adds a withdrawal fee TLV extension to the vault. Must be called
    /// before vault initialization. Requires authority signature.
    pub fn initialize_withdrawal_fee(
        ctx: Context<InitWithdrawalFee>,
        args: InitWithdrawalFeeArgs,
    ) -> Result<()> {
        instructions::initialize_withdrawal_fee::handler(ctx, args)
    }

    /// Updates an existing deposit fee extension. The fee must have been
    /// previously initialized. Requires authority signature.
    pub fn update_deposit_fee(
        ctx: Context<UpdateDepositFee>,
        args: UpdateDepositFeeArgs,
    ) -> Result<()> {
        instructions::update_deposit_fee::handler(ctx, args)
    }

    /// Updates an existing withdrawal fee extension. The fee must have been
    /// previously initialized. Requires authority signature.
    pub fn update_withdrawal_fee(
        ctx: Context<UpdateWithdrawalFee>,
        args: UpdateWithdrawalFeeArgs,
    ) -> Result<()> {
        instructions::update_withdrawal_fee::handler(ctx, args)
    }

    /// Updates the vault nav and increases nav version by 1
    /// Requires authority signature.
    pub fn update_vault_nav(ctx: Context<UpdateVaultNav>, updated_nav: u128) -> Result<()> {
        instructions::update_nav::handler(ctx, updated_nav)
    }

    /// Creates a deposit request with state pending (Pending vault authority acceptance)
    pub fn create_deposit_request<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateDepositRequest<'info>>,
        args: RequestArgs,
    ) -> Result<()> {
        instructions::create_deposit_request::handler(ctx, args)
    }
}

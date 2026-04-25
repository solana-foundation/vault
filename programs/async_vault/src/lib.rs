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

    /// Stores a pending authority on the vault without transferring control.
    /// The new authority must later call `accept_authority_invitation` to
    /// complete the transfer. Requires current authority signature.
    pub fn invite_new_authority(
        ctx: Context<InviteNewAuthority>,
        args: InviteNewAuthorityArgs,
    ) -> Result<()> {
        instructions::invite_new_authority::handler(ctx, args)
    }

    /// Completes the authority transfer by setting the vault authority to
    /// the pending authority. Requires both the current and new authority
    /// to sign.
    pub fn accept_authority_invitation(ctx: Context<AcceptAuthorityInvitation>) -> Result<()> {
        instructions::accept_authority_invitation::handler(ctx)
    }
}

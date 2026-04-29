use anchor_lang::prelude::*;

pub mod error;
pub mod extensions;
pub mod instructions;
pub mod state;
pub mod utils;

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

    /// Creates a deposit request with state pending (Pending vault authority acceptance)
    pub fn create_deposit_request<'info>(
        ctx: Context<CreateDepositRequest>,
        args: RequestArgs,
    ) -> Result<()> {
        instructions::create_deposit_request::handler(ctx, args)
    }

    /// Creates a redeem request with state pending (Pending vault authority acceptance)
    pub fn create_redeem_request(
        ctx: Context<CreateRedeemRequest>,
        args: RequestArgs,
    ) -> Result<()> {
        instructions::create_redeem_request::handler(ctx, args)
    }

    /// User claims their shares or assets from an approved Deposit or Redemption request.
    /// Request must be Claimable.
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        instructions::claim::handler(ctx)
    }

    /// It sets an operator for the vault.
    /// Requires Request owner signature.
    pub fn set_operator(ctx: Context<SetOperator>) -> Result<()> {
        instructions::set_operator::handler(ctx)
    }

    /* Vault Authority instructions */

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
    /// Updates the async vault configuration. Can modify paused status,
    /// async inflows, and async outflows. Requires authority signature.
    pub fn update_vault(ctx: Context<UpdateVault>, paused: bool) -> Result<()> {
        instructions::update_vault::handler(ctx, paused)
    }
    /// Updates the vault nav and increases nav version by 1
    /// Requires authority signature.
    pub fn update_vault_nav(ctx: Context<UpdateVaultNav>, updated_nav: u128) -> Result<()> {
        instructions::update_nav::handler(ctx, updated_nav)
    }

    /// Approve a pending request, allowing the User to execute the Claim instruction.
    /// This sets the Request's claimable NAV to the Vault's current NAV.
    pub fn approve_request(ctx: Context<ApproveRequest>) -> Result<()> {
        instructions::approve_request::handler(ctx)
    }
}

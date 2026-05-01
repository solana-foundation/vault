use anchor_lang::prelude::*;

pub mod error;
pub mod extensions;
pub mod instructions;
pub mod state;
pub mod utils;

use extensions::*;
use instructions::*;

declare_id!("2kUpRoU8oGpstygkk3ZE51upGSq9UpkjNoEUiiQ88MMY");

#[program]
pub mod async_vault {
    use super::*;

    /* Vault Authority instructions */

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

    /// User claims their shares or assets from an approved Deposit or Redemption request.
    /// Request must be Claimable.
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        instructions::claim::handler(ctx)
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
    pub fn approve_request<'info>(
        ctx: Context<'_, '_, '_, 'info, ApproveRequest<'info>>,
    ) -> Result<()> {
        instructions::approve_request::handler(ctx)
    }

    /// Reject a pending request. For deposit requests, the deposited assets are
    /// refunded to the user. For redeem requests, the shares are minted back to the user.
    /// The request account is closed and its rent is returned to the user.
    pub fn reject_request(ctx: Context<RejectRequest>) -> Result<()> {
        instructions::reject_request::handler(ctx)
    }

    /* EXTENSION INSTRUCTIONS */

    /// Adds a deposit fee TLV extension to the vault. Must be called
    /// before vault initialization. Requires authority signature.
    pub fn initialize_deposit_fee(
        ctx: Context<InitDepositFee>,
        args: InitDepositFeeArgs,
    ) -> Result<()> {
        extensions::fee::instructions::initialize_deposit_fee::handler(ctx, args)
    }

    /// Adds a withdrawal fee TLV extension to the vault. Must be called
    /// before vault initialization. Requires authority signature.
    pub fn initialize_withdrawal_fee(
        ctx: Context<InitWithdrawalFee>,
        args: InitWithdrawalFeeArgs,
    ) -> Result<()> {
        extensions::fee::instructions::initialize_withdrawal_fee::handler(ctx, args)
    }

    /// Updates an existing deposit fee extension. The fee must have been
    /// previously initialized. Requires authority signature.
    pub fn update_deposit_fee(
        ctx: Context<UpdateDepositFee>,
        args: UpdateDepositFeeArgs,
    ) -> Result<()> {
        extensions::fee::instructions::update_deposit_fee::handler(ctx, args)
    }

    /// Updates an existing withdrawal fee extension. The fee must have been
    /// previously initialized. Requires authority signature.
    pub fn update_withdrawal_fee(
        ctx: Context<UpdateWithdrawalFee>,
        args: UpdateWithdrawalFeeArgs,
    ) -> Result<()> {
        extensions::fee::instructions::update_withdrawal_fee::handler(ctx, args)
    }

    /* USER INSTRUCTIONS */

    /// Creates a deposit request with state pending (Pending vault authority acceptance)
    pub fn create_deposit_request(
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

    /// Cancels a pending request. For deposits, refunds the full amount
    /// back to the user. For redemptions, mints the shares back.
    pub fn cancel_request(ctx: Context<CancelRequest>) -> Result<()> {
        instructions::cancel_request::handler(ctx)
    }

    /// Withdraws assets from the vault reserve to a specified token account.
    /// Used for async operations such as deploying assets offchain.
    /// Requires authority signature.
    pub fn withdraw_assets(ctx: Context<WithdrawAssets>, amount: u64) -> Result<()> {
        instructions::withdraw_assets::handler(ctx, amount)
    }

    /// Sets an operator for the Request.
    /// Requires Request owner signature.
    pub fn set_operator(ctx: Context<SetOperator>) -> Result<()> {
        instructions::set_operator::handler(ctx)
    }
}

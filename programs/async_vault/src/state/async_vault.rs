use anchor_lang::prelude::*;
use vault_common::{FeeType, VaultProgramError};

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType},
};

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub asset_mint_address: Pubkey,
    /// share mint address
    pub share_mint_address: Pubkey,
    /// token account holding confirmed vault assets
    pub vault_token_account: Pubkey,
    /// authority that can sign permissioned instructions
    pub authority: Pubkey,
    /// pubkey that is required to own the TokenAccount fees are sent to
    pub fee_recipient: Pubkey,
    /// initial price of shares in asset units (scaled by asset mint decimals)
    pub initial_price: u64,
    /// paused
    pub paused: bool,
    /// once a vault is initialized, no extensions can be added
    pub initialized: bool,
    /// token account holding assets from deposits awaiting share issuance
    pub pending_vault: Pubkey,
    /// net asset value (assets per share), default 0 until first NAV update
    pub nav: u128,
    /// nav version, incremented on each NAV update
    pub nav_version: u64,
    /// whether deposits are processed asynchronously
    pub async_inflows: bool,
    /// whether withdrawals are processed asynchronously
    pub async_outflows: bool,
    /// count of pending async deposit/withdrawal requests
    pub pending_async_requests: u16,
    // Keeps track of total requests
    pub request_counter: u64,
    /// virtual vault asset balance, accounts for tokens that may
    /// have been withdrawn by the vault authority
    pub total_asset_balance: u64,
    pub reserve_bump: u8,
    pub pending_vault_bump: u8,
    pub bump: u8,
}

impl Vault {
    pub const TLV_START: usize = 8 + Self::INIT_SPACE;

    pub fn get_deposit_fee(&mut self, account_data: &[u8], amount: u64) -> Result<u64> {
        match Self::get_fee_extension(account_data, ExtensionType::DepositFee)? {
            Some(fee) => fee.get_fee(amount),
            None => Ok(0),
        }
    }

    pub fn get_withdrawal_fee(account_data: &[u8], amount: u64) -> Result<u64> {
        match Self::get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
            Some(fee) => fee.get_fee(amount),
            None => Ok(0),
        }
    }

    pub fn calculate_deposit_fee_when_minting(account_data: &[u8], net_assets: u64) -> Result<u64> {
        match Self::get_fee_extension(account_data, ExtensionType::DepositFee)? {
            Some(fee) => fee.get_deposit_fee_when_minting(net_assets),
            None => Ok(0),
        }
    }

    pub fn calculate_withdraw_fee_when_redeeming(
        account_data: &[u8],
        gross_assets: u64,
    ) -> Result<u64> {
        match Self::get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
            Some(fee) => fee.get_withdraw_fee_when_redeeming(gross_assets),
            None => Ok(0),
        }
    }

    pub fn assert_unpaused_and_initialized(&self) -> Result<()> {
        require!(self.initialized, AsyncVaultError::UninitializedVault);
        require!(!self.paused, AsyncVaultError::PausedVault);
        Ok(())
    }

    pub fn assert_uninitialized(&self) -> Result<()> {
        require!(!self.initialized, AsyncVaultError::VaultAlreadyInitialized);
        Ok(())
    }

    fn get_fee_extension(account_data: &[u8], ext_type: ExtensionType) -> Result<Option<FeeType>> {
        if account_data.len() <= Self::TLV_START {
            return Ok(None);
        }
        let tlv_data = &account_data[Self::TLV_START..];
        match extensions::get_extension_bytes(tlv_data, ext_type) {
            Some(bytes) => {
                let mut slice = bytes;
                let fee = FeeType::deserialize(&mut slice)
                    .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;
                Ok(Some(fee))
            }
            None => Ok(None),
        }
    }

    pub fn calculate_shares(&mut self, decimals: u8, net_amount: u64) -> Result<u64> {
        let precision = 10u128.pow(decimals as u32);
        let shares = u128::from(net_amount)
            .checked_mul(precision)
            .ok_or(VaultProgramError::ArithmeticError)?
            .checked_div(self.nav)
            .ok_or(VaultProgramError::ArithmeticError)?;
        Ok(u64::try_from(shares).map_err(|_| VaultProgramError::ArithmeticError)?)
    }
}

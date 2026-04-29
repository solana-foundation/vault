use anchor_lang::prelude::*;
use vault_common::VaultProgramError;

use crate::error::AsyncVaultError;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub asset_mint: Pubkey,
    /// share mint address
    pub share_mint: Pubkey,
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
    /// virtual vault asset balance, accounts for tokens that may
    /// have been withdrawn by the vault authority
    pub total_asset_balance: u64,
    pub reserve_bump: u8,
    pub pending_vault_bump: u8,
    pub bump: u8,
    // Used for updating the vault authority (New Authority)
    pub pending_authority: Option<Pubkey>,
}

impl Vault {
    pub fn assert_unpaused_and_initialized(&self) -> Result<()> {
        require!(self.initialized, AsyncVaultError::UninitializedVault);
        require!(!self.paused, AsyncVaultError::PausedVault);
        Ok(())
    }

    pub fn assert_uninitialized(&self) -> Result<()> {
        require!(!self.initialized, AsyncVaultError::VaultAlreadyInitialized);
        Ok(())
    }

    /// Converts an asset amount into shares using the current NAV.
    ///
    /// `shares = net_amount * 10^decimals / nav`
    pub fn calculate_shares(&mut self, decimals: u8, net_amount: u64) -> Result<u64> {
        let precision = 10u128
            .checked_pow(decimals as u32)
            .ok_or(VaultProgramError::ArithmeticError)?;
        let shares = u128::from(net_amount)
            .checked_mul(precision)
            .ok_or(VaultProgramError::ArithmeticError)?
            .checked_div(self.nav)
            .ok_or(VaultProgramError::ArithmeticError)?;
        Ok(u64::try_from(shares).map_err(|_| VaultProgramError::ArithmeticError)?)
    }

    /// Converts a share amount into assets using the current NAV.
    ///
    /// `assets = share_amount * nav / 10^decimals`
    pub fn calculate_assets(&self, decimals: u8, share_amount: u64) -> Result<u64> {
        let precision = 10u128
            .checked_pow(decimals as u32)
            .ok_or(VaultProgramError::ArithmeticError)?;
        let assets = u128::from(share_amount)
            .checked_mul(self.nav)
            .ok_or(VaultProgramError::ArithmeticError)?
            .checked_div(precision)
            .ok_or(VaultProgramError::ArithmeticError)?;
        Ok(u64::try_from(assets).map_err(|_| VaultProgramError::ArithmeticError)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with_nav(nav: u128) -> Vault {
        Vault {
            asset_mint: Pubkey::default(),
            share_mint: Pubkey::default(),
            vault_token_account: Pubkey::default(),
            authority: Pubkey::default(),
            fee_recipient: Pubkey::default(),
            initial_price: 1_000_000,
            paused: false,
            initialized: true,
            pending_vault: Pubkey::default(),

            nav,
            nav_version: 1,
            async_inflows: true,
            async_outflows: true,
            pending_async_requests: 0,
            total_asset_balance: 0,
            reserve_bump: 0,
            pending_vault_bump: 0,

            bump: 0,
            pending_authority: None,
        }
    }

    #[test]
    fn calculate_shares_one_to_one() {
        let mut vault = vault_with_nav(1_000_000);
        let shares = vault.calculate_shares(6, 1_000_000).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn calculate_shares_nav_above_one() {
        let mut vault = vault_with_nav(2_000_000);
        let shares = vault.calculate_shares(6, 2_000_000).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn calculate_shares_fractional_result_truncates() {
        let mut vault = vault_with_nav(3_000_000);
        let shares = vault.calculate_shares(6, 1_000_000).unwrap();
        // 1_000_000 * 1e6 / 3_000_000 = 333_333.333… → truncated to 333_333
        assert_eq!(shares, 333_333);
    }

    #[test]
    fn calculate_shares_zero_amount() {
        let mut vault = vault_with_nav(1_000_000);
        let shares = vault.calculate_shares(6, 0).unwrap();
        assert_eq!(shares, 0);
    }

    #[test]
    fn calculate_shares_different_decimals() {
        let mut vault = vault_with_nav(100_000_000);
        // 8 decimals: 1 token = 100_000_000 units
        let shares = vault.calculate_shares(8, 100_000_000).unwrap();
        assert_eq!(shares, 100_000_000);
    }

    #[test]
    fn calculate_shares_zero_nav_errors() {
        let mut vault = vault_with_nav(0);
        assert!(vault.calculate_shares(6, 1_000_000).is_err());
    }

    #[test]
    fn calculate_shares_large_amount_no_overflow() {
        let mut vault = vault_with_nav(1_000_000);
        let shares = vault.calculate_shares(6, u64::MAX).unwrap();
        assert_eq!(shares, u64::MAX);
    }
}

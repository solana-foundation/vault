use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        self,
        extension::{BaseStateWithExtensions, StateWithExtensions},
    },
    token_interface::{self, Mint, TokenInterface, TransferChecked},
};
use vault_common::FeeType;

use crate::{
    error::AsyncVaultError,
    extensions::{self, ExtensionType},
    state::{Vault, VAULT_CONFIG_SEED},
};

pub const TLV_START: usize = 8 + 241;

pub fn get_fee_extension(account_data: &[u8], ext_type: ExtensionType) -> Result<Option<FeeType>> {
    if account_data.len() <= TLV_START {
        return Ok(None);
    }
    let tlv_data = &account_data[TLV_START..];
    match extensions::get_extension_bytes(tlv_data, ext_type)? {
        Some(bytes) => {
            let mut slice = bytes;
            let fee = FeeType::deserialize(&mut slice)
                .map_err(|_| error!(AsyncVaultError::InvalidExtensionData))?;
            Ok(Some(fee))
        }
        None => Ok(None),
    }
}

pub fn get_deposit_fee(account_data: &[u8], amount: u64) -> Result<Option<u64>> {
    match get_fee_extension(account_data, ExtensionType::DepositFee)? {
        Some(fee) => Ok(Some(fee.get_fee(amount)?)),
        None => Ok(None),
    }
}

pub fn transfer_fee_to_recipient<'info>(
    fee_recipient_info: &AccountInfo<'info>,
    from: AccountInfo<'info>,
    vault: &Account<'info, Vault>,
    asset_mint: &InterfaceAccount<'info, Mint>,
    asset_token_program: &Interface<'info, TokenInterface>,
    share_mint_key: Pubkey,
    fee: u64,
) -> Result<()> {
    require!(
        fee_recipient_info.owner == asset_token_program.key,
        AsyncVaultError::InvalidFeeRecipient
    );

    let fee_recipient_data = fee_recipient_info.try_borrow_data()?;
    let fee_recipient_state =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&fee_recipient_data)?;
    require!(
        fee_recipient_state.base.owner == vault.fee_recipient,
        AsyncVaultError::InvalidFeeRecipient
    );
    require!(
        fee_recipient_state.base.mint == asset_mint.key(),
        AsyncVaultError::InvalidFeeRecipient
    );
    drop(fee_recipient_data);

    let seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, share_mint_key.as_ref(), &[vault.bump]]];
    let cpi_accounts = TransferChecked {
        from,
        mint: asset_mint.to_account_info(),
        to: fee_recipient_info.clone(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(asset_token_program.to_account_info(), cpi_accounts, seeds);
    token_interface::transfer_checked(cpi_ctx, fee, asset_mint.decimals)
}

pub fn calculate_deposit_fee_when_minting(account_data: &[u8], net_assets: u64) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::DepositFee)? {
        Some(fee) => fee.get_deposit_fee_when_minting(net_assets),
        None => Ok(0),
    }
}

pub fn get_withdrawal_fee(account_data: &[u8], amount: u64) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
        Some(fee) => fee.get_fee(amount),
        None => Ok(0),
    }
}

pub fn calculate_withdraw_fee_when_redeeming(
    account_data: &[u8],
    gross_assets: u64,
) -> Result<u64> {
    match get_fee_extension(account_data, ExtensionType::WithdrawalFee)? {
        Some(fee) => fee.get_withdraw_fee_when_redeeming(gross_assets),
        None => Ok(0),
    }
}

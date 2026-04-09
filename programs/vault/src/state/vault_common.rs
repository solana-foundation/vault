use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum SwapKind {
    Deposit,
    Mint,
    Withdraw,
    Redeem,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct SwapParams {
    /// Amount being swapped based on the Swap IX variant:
    /// - Deposit: asset amount in
    /// - Mint: share amount out
    /// - Withdraw: asset amount out
    /// - Redeem: share amount in
    pub amount: u64,
    /// Slippage threshold:
    /// - Deposit: min shares out
    /// - Mint: max assets in
    /// - Withdraw: max shares burned
    /// - Redeem: min assets out
    pub threshold_amount: u64,
}

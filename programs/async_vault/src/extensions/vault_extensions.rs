use crate::extensions::TLV_HEADER_SIZE;

pub const MAX_FEE_DATA_SIZE: usize = 9;
pub const FEE_TLV_SIZE: usize = TLV_HEADER_SIZE + MAX_FEE_DATA_SIZE;

pub const PAUSABLE_SUBSCRIPTIONS_DATA_SIZE: usize = 1;
pub const PAUSABLE_SUBSCRIPTIONS_TLV_SIZE: usize =
    TLV_HEADER_SIZE + PAUSABLE_SUBSCRIPTIONS_DATA_SIZE;

pub const PAUSABLE_REDEMPTIONS_DATA_SIZE: usize = 1;
pub const PAUSABLE_REDEMPTIONS_TLV_SIZE: usize = TLV_HEADER_SIZE + PAUSABLE_REDEMPTIONS_DATA_SIZE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum ExtensionType {
    DepositFee = 1,
    WithdrawalFee = 2,
    PausableSubscriptions = 3,
    PausableRedemptions = 4,
}

impl ExtensionType {
    pub fn try_from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::DepositFee),
            2 => Some(Self::WithdrawalFee),
            3 => Some(Self::PausableSubscriptions),
            4 => Some(Self::PausableRedemptions),
            _ => None,
        }
    }

    pub fn data_len(&self) -> usize {
        match self {
            Self::DepositFee | Self::WithdrawalFee => MAX_FEE_DATA_SIZE,
            Self::PausableSubscriptions => PAUSABLE_SUBSCRIPTIONS_DATA_SIZE,
            Self::PausableRedemptions => PAUSABLE_REDEMPTIONS_DATA_SIZE,
        }
    }

    pub fn tlv_len(&self) -> usize {
        TLV_HEADER_SIZE + self.data_len()
    }
}

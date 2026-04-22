pub const TLV_TYPE_SIZE: usize = 2;
pub const TLV_LENGTH_SIZE: usize = 2;
pub const TLV_HEADER_SIZE: usize = TLV_TYPE_SIZE + TLV_LENGTH_SIZE;
pub const MAX_FEE_DATA_SIZE: usize = 9;
pub const FEE_TLV_SIZE: usize = TLV_HEADER_SIZE + MAX_FEE_DATA_SIZE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum ExtensionType {
    DepositFee = 1,
    WithdrawalFee = 2,
}

impl ExtensionType {
    pub fn try_from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::DepositFee),
            2 => Some(Self::WithdrawalFee),
            _ => None,
        }
    }

    pub fn data_len(&self) -> usize {
        match self {
            Self::DepositFee | Self::WithdrawalFee => MAX_FEE_DATA_SIZE,
        }
    }

    pub fn tlv_len(&self) -> usize {
        TLV_HEADER_SIZE + self.data_len()
    }
}

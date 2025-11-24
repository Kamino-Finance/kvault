use anchor_lang::prelude::*;
#[cfg(feature = "serde")]
use strum::EnumIter;
use strum::EnumString;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Clone, Copy, Debug, EnumString)]
#[cfg_attr(feature = "serde", derive(EnumIter))]
pub enum UpdateGlobalConfigMode {
    PendingAdmin(Pubkey),
    MinWithdrawalPenaltyLamports(u64),
    MinWithdrawalPenaltyBPS(u64),
}

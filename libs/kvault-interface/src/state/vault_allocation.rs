use bytemuck::{Pod, Zeroable};
use solana_pubkey::Pubkey;

use super::pod::PodU128;

/// Per-reserve allocation within a vault.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct VaultAllocation {
    /// Klend reserve address this allocation targets. `Pubkey::default()` means the slot is empty.
    pub reserve: Pubkey,
    /// Token account holding the cTokens received from the reserve.
    pub ctoken_vault: Pubkey,
    /// Relative weight for target allocation distribution.
    pub target_allocation_weight: u64,
    /// Maximum tokens that can be invested in this reserve.
    pub token_allocation_cap: u64,
    /// Bump seed for the [`ctoken_vault`](Self::ctoken_vault) PDA.
    pub ctoken_vault_bump: u64,
    /// Maximum cTokens that can be invested in this reserve.
    ///
    /// `0` and `u64::MAX` mean uncapped. For finite values, the
    /// effective token-denominated cap is
    /// `min(token_allocation_cap, ctoken_allocation_cap * reserve_exchange_rate)`.
    pub ctoken_allocation_cap: u64,
    /// Reserved for future configuration fields.
    pub config_padding: [u64; 126],
    /// Current cToken balance invested in this reserve.
    pub ctoken_allocation: u64,
    /// Slot number of the last invest call for this reserve.
    pub last_invest_slot: u64,
    /// Target token allocation as a scaled fraction ([`Fraction`](crate::Fraction)).
    pub token_target_allocation_sf: PodU128,
    /// Reserved for future state fields.
    pub state_padding: [u64; 128],
}

const _: () = assert!(core::mem::size_of::<VaultAllocation>() == 2160);

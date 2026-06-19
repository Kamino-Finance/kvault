use bytemuck::{Pod, Zeroable};

/// Reward distribution state for a vault.
///
/// Tracks the per-second reward rate, last issuance timestamp, and the
/// pool of rewards that has been topped up but not yet distributed to
/// the vault's available liquidity.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct VaultRewardInfo {
    /// Rewards distributed per second (in underlying token units).
    pub reward_per_second: u64,
    /// Unix timestamp of the last reward issuance.
    pub last_issuance_ts: u64,
    /// Rewards topped up but not yet moved to [`VaultState::token_available`](super::VaultState::token_available).
    pub rewards_available: u64,
    /// Cumulative rewards distributed (analytics only — does not affect accounting).
    pub cumulative_rewards_distributed_analytics: u64,

    /// Reserved for future use.
    pub padding: [u64; 8],
}

const _: () = assert!(core::mem::size_of::<VaultRewardInfo>() == 96);

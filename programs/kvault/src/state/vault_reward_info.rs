use anchor_lang::prelude::*;

use crate::utils::consts::VAULT_REWARD_INFO_SIZE;

static_assertions::const_assert_eq!(
    VAULT_REWARD_INFO_SIZE,
    std::mem::size_of::<VaultRewardInfo>()
);
static_assertions::const_assert_eq!(0, std::mem::size_of::<VaultRewardInfo>() % 8);

#[zero_copy]
#[derive(AnchorDeserialize, Debug, PartialEq, Eq, Default)]
pub struct VaultRewardInfo {
    pub reward_per_second: u64,
    pub last_issuance_ts: u64,


    pub rewards_available: u64,

    pub cumulative_rewards_distributed_analytics: u64,

    pub padding: [u64; 8],
}

impl VaultRewardInfo {
    pub fn has_active_rewards(&self) -> bool {
        self.reward_per_second > 0 && self.rewards_available > 0
    }
}

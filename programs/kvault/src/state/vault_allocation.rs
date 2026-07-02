use anchor_lang::prelude::*;
use kamino_lending::fraction::Fraction;

use crate::{
    operations::vault_operations::common::InvestedReserve, utils::consts::VAULT_ALLOCATION_SIZE,
};

use super::CtokenCap;

static_assertions::const_assert_eq!(
    VAULT_ALLOCATION_SIZE,
    std::mem::size_of::<VaultAllocation>()
);
static_assertions::const_assert_eq!(0, std::mem::size_of::<VaultAllocation>() % 16);

#[zero_copy]
#[derive(AnchorDeserialize, Debug, PartialEq, Eq)]
pub struct VaultAllocation {
    pub reserve: Pubkey,
    pub ctoken_vault: Pubkey,
    pub target_allocation_weight: u64,

    pub token_allocation_cap: u64,
    pub ctoken_vault_bump: u64,
    pub ctoken_allocation_cap: u64,

   
    pub config_padding: [u64; 126],

    pub ctoken_allocation: u64,
    pub last_invest_slot: u64,
    pub token_target_allocation_sf: u128,

    pub state_padding: [u64; 128],
}

impl VaultAllocation {
    pub fn get_token_target_allocation(&self) -> Fraction {
        Fraction::from_bits(self.token_target_allocation_sf)
    }

    pub fn set_token_target_allocation(&mut self, token_target_allocation: Fraction) {
        self.token_target_allocation_sf = token_target_allocation.to_bits();
    }

    pub fn can_be_removed(&self) -> bool {
       
        self.ctoken_allocation == 0 && self.target_allocation_weight == 0
    }

    pub const fn ctoken_allocation_cap(&self) -> CtokenCap {
        CtokenCap::new(self.ctoken_allocation_cap)
    }

    pub fn effective_token_allocation_cap(&self, invested: &InvestedReserve) -> Fraction {
        let token_cap = Fraction::from(self.token_allocation_cap);
        token_cap.min(invested.ctoken_cap_in_liquidity)
    }

    pub fn set_last_invest_slot(&mut self, slot: u64) {
        self.last_invest_slot = slot;
    }
}

impl Default for VaultAllocation {
    fn default() -> Self {
        Self {
            reserve: Pubkey::default(),
            ctoken_vault: Pubkey::default(),
            target_allocation_weight: 0,
            ctoken_allocation: 0,
            token_target_allocation_sf: 0,
            token_allocation_cap: u64::MAX,
            ctoken_allocation_cap: u64::MAX,
            last_invest_slot: 0,
            ctoken_vault_bump: 0,
            config_padding: [0; 126],
            state_padding: [0; 128],
        }
    }
}

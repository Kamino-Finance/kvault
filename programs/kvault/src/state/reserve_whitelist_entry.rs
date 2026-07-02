use anchor_lang::prelude::*;

use crate::utils::consts::RESERVE_WHITELIST_ENTRY_SIZE;

static_assertions::const_assert_eq!(
    RESERVE_WHITELIST_ENTRY_SIZE,
    std::mem::size_of::<ReserveWhitelistEntry>()
);
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveWhitelistEntry>() % 8);

#[account]
pub struct ReserveWhitelistEntry {





    pub token_mint: Pubkey,
    pub reserve: Pubkey,
    pub whitelist_add_allocation: u8,
    pub whitelist_invest: u8,
    pub padding: [u8; 62],
}

impl ReserveWhitelistEntry {
    pub fn is_add_allocation_whitelisted(&self) -> bool {
        if self.whitelist_add_allocation > 1 {
           
            panic!(
                "Invalid {} value for whitelist_add_allocation, it should be 0 or 1",
                self.whitelist_add_allocation
            );
        }
        self.whitelist_add_allocation == 1
    }

    pub fn is_invest_whitelisted(&self) -> bool {
        if self.whitelist_invest > 1 {
           
            panic!(
                "Invalid {} value for whitelist_invest, it should be 0 or 1",
                self.whitelist_invest
            );
        }
        self.whitelist_invest == 1
    }
}

impl Default for ReserveWhitelistEntry {
    fn default() -> Self {
        Self {
            token_mint: Pubkey::default(),
            reserve: Pubkey::default(),
            whitelist_add_allocation: 0,
            whitelist_invest: 0,
            padding: [0; 62],
        }
    }
}

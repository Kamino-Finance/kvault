use anchor_lang::prelude::*;
use bytemuck::Zeroable;

use crate::{
    utils::{
        consts::{GLOBAL_CONFIG_SIZE, MAX_WITHDRAWAL_PENALTY_BPS, MAX_WITHDRAWAL_PENALTY_LAMPORTS},
        global_config::UpdateGlobalConfigMode,
    },
    KaminoVaultError,
};

static_assertions::const_assert_eq!(GLOBAL_CONFIG_SIZE, std::mem::size_of::<GlobalConfig>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<GlobalConfig>() % 8);

#[account(zero_copy)]
#[derive(AnchorDeserialize, PartialEq, Eq)]
#[repr(C)]
pub struct GlobalConfig {
    pub global_admin: Pubkey,
    pub pending_admin: Pubkey,

    pub withdrawal_penalty_lamports: u64,
    pub withdrawal_penalty_bps: u64,

    pub padding: [u8; 944],
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl GlobalConfig {
    pub fn init(&mut self, initial_admin: Pubkey) {
        self.global_admin = initial_admin;
        self.pending_admin = initial_admin;
        self.withdrawal_penalty_bps = 0;
        self.withdrawal_penalty_lamports = 0;
    }

    pub fn update_value(&mut self, update: UpdateGlobalConfigMode) -> Result<()> {
        let global_config = self;

        msg!("Updating global config with mode {:?}", update);
        match update {
            UpdateGlobalConfigMode::PendingAdmin(new_admin) => {
                msg!("Prv value is: {:?}", global_config.pending_admin);
                msg!("New value is: {:?}", new_admin);
                global_config.pending_admin = new_admin;
            }
            UpdateGlobalConfigMode::MinWithdrawalPenaltyLamports(new_value) => {
                require_gte!(
                    MAX_WITHDRAWAL_PENALTY_LAMPORTS,
                    new_value,
                    KaminoVaultError::WithdrawalFeeLamportsGreaterThanMaxAllowed
                );
                msg!(
                    "Prv value is: {:?}",
                    global_config.withdrawal_penalty_lamports
                );
                msg!("New value is: {:?}", new_value);
                global_config.withdrawal_penalty_lamports = new_value;
            }
            UpdateGlobalConfigMode::MinWithdrawalPenaltyBPS(new_value) => {
                require_gte!(
                    MAX_WITHDRAWAL_PENALTY_BPS,
                    new_value,
                    KaminoVaultError::WithdrawalFeeBPSGreaterThanMaxAllowed
                );
                msg!("Prv value is: {:?}", global_config.withdrawal_penalty_bps);
                msg!("New value is: {:?}", new_value);
                global_config.withdrawal_penalty_bps = new_value;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn apply_pending_admin(&mut self) -> Result<()> {
        self.global_admin = self.pending_admin;
        Ok(())
    }
}

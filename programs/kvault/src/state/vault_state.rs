use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use kamino_lending::{fraction::Fraction, utils::FractionExtra};

use crate::{
    operations::vault_operations::common::Invested, utils::consts::VAULT_STATE_SIZE,
    KaminoVaultError,
};

use super::{CtokenCap, VaultAllocation, VaultRewardInfo};

pub const MAX_RESERVES: usize = 25;

static_assertions::const_assert_eq!(VAULT_STATE_SIZE, std::mem::size_of::<VaultState>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<VaultState>() % 16);

#[account(zero_copy)]
#[derive(AnchorDeserialize, PartialEq, Eq)]
pub struct VaultState {
   
    pub vault_admin_authority: Pubkey,

    pub base_vault_authority: Pubkey,
    pub base_vault_authority_bump: u64,

    pub token_mint: Pubkey,
    pub token_mint_decimals: u64,
    pub token_vault: Pubkey,
    pub token_program: Pubkey,

   
    pub shares_mint: Pubkey,
    pub shares_mint_decimals: u64,

   
    pub token_available: u64,
    pub shares_issued: u64,

    pub available_crank_funds: u64,
    pub unallocated_weight: u64,

    pub performance_fee_bps: u64,
    pub management_fee_bps: u64,
    pub last_fee_charge_timestamp: u64,
    pub prev_aum_sf: u128,
   
    pub pending_fees_sf: u128,

    pub vault_allocation_strategy: [VaultAllocation; MAX_RESERVES],
    pub padding_1: [u128; 256],

   
    pub min_deposit_amount: u64,
    pub min_withdraw_amount: u64,
    pub min_invest_amount: u64,
    pub min_invest_delay_slots: u64,
    pub crank_fund_fee_per_reserve: u64,

    pub pending_admin: Pubkey,

    pub cumulative_earned_interest_sf: u128,
    pub cumulative_mgmt_fees_sf: u128,
    pub cumulative_perf_fees_sf: u128,

    pub name: [u8; 40],
    pub vault_lookup_table: Pubkey,
    pub vault_farm: Pubkey,

    pub creation_timestamp: u64,

   
    pub unallocated_tokens_cap: u64,
    pub allocation_admin: Pubkey,

    pub withdrawal_penalty_lamports: u64,
    pub withdrawal_penalty_bps: u64,

    pub first_loss_capital_farm: Pubkey,

    pub allow_allocations_in_whitelisted_reserves_only: u8,
    pub allow_invest_in_whitelisted_reserves_only: u8,

    pub padding_2: [u8; 6],

    pub deposit_cap: u64,
    pub reward_info: VaultRewardInfo,

    pub padding_3: [u128; 232],
}

impl Default for VaultState {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl VaultState {
    pub fn get_pending_fees(&self) -> Fraction {
        Fraction::from_bits(self.pending_fees_sf)
    }

    pub fn set_pending_fees(&mut self, pending_fees: Fraction) {
        self.pending_fees_sf = pending_fees.to_bits();
    }

    pub fn get_prev_aum(&self) -> Fraction {
        Fraction::from_bits(self.prev_aum_sf)
    }

    pub fn set_prev_aum(&mut self, current_aum: Fraction) {
        self.prev_aum_sf = current_aum.to_bits();
    }

    pub fn get_reserves_count(&self) -> usize {
        self.vault_allocation_strategy
            .iter()
            .filter(|r| r.reserve != Pubkey::default())
            .count()
    }

    pub fn get_reserves_with_allocation_count(&self) -> usize {
        self.vault_allocation_strategy
            .iter()
            .filter(|r| {
                r.reserve != Pubkey::default()
                    && r.target_allocation_weight > 0
                    && r.token_allocation_cap > 0
            })
            .count()
    }

    pub fn get_cumulative_earned_interest(&self) -> Fraction {
        Fraction::from_bits(self.cumulative_earned_interest_sf)
    }

    pub fn set_cumulative_earned_interest(&mut self, cumulative_earned_interest: Fraction) {
        self.cumulative_earned_interest_sf = cumulative_earned_interest.to_bits();
    }

    pub fn get_cumulative_mgmt_fees(&self) -> Fraction {
        Fraction::from_bits(self.cumulative_mgmt_fees_sf)
    }

    pub fn set_cumulative_mgmt_fees(&mut self, cumulative_mgmt_fees: Fraction) {
        self.cumulative_mgmt_fees_sf = cumulative_mgmt_fees.to_bits();
    }

    pub fn get_cumulative_perf_fees(&self) -> Fraction {
        Fraction::from_bits(self.cumulative_perf_fees_sf)
    }

    pub fn set_cumulative_perf_fees(&mut self, cumulative_perf_fees: Fraction) {
        self.cumulative_perf_fees_sf = cumulative_perf_fees.to_bits();
    }

    pub fn vault_allows_allocations_in_whitelisted_reserves_only(&self) -> bool {
        if self.allow_allocations_in_whitelisted_reserves_only > 1 {
           
            panic!(
                "Invalid {} value for allow_allocations_in_whitelisted_reserves_only, it should be 0 or 1",
                self.allow_allocations_in_whitelisted_reserves_only
            );
        }
        self.allow_allocations_in_whitelisted_reserves_only == 1
    }

    pub fn vault_allows_invest_in_whitelisted_reserves_only(&self) -> bool {
        if self.allow_invest_in_whitelisted_reserves_only > 1 {
           
            panic!(
                "Invalid {} value for allow_invest_in_whitelisted_reserves_only, it should be 0 or 1",
                self.allow_invest_in_whitelisted_reserves_only
            );
        }
        self.allow_invest_in_whitelisted_reserves_only == 1
    }

    pub fn compute_aum(&self, invested_total: &Fraction) -> Result<Fraction> {
       
        let pending_fees = self.get_pending_fees();

        if Fraction::from(self.token_available) + invested_total < pending_fees {
            return err!(KaminoVaultError::AUMBelowPendingFees);
        }

        Ok(Fraction::from(self.token_available) + invested_total - pending_fees)
    }

    pub fn validate(&self) -> Result<()> {
        if self.vault_admin_authority == Pubkey::default() {
            return err!(KaminoVaultError::AdminAuthorityIncorrect);
        }

        if self.base_vault_authority == Pubkey::default() {
            return err!(KaminoVaultError::BaseVaultAuthorityIncorrect);
        }

        if self.base_vault_authority_bump > u64::from(u8::MAX) {
            return err!(KaminoVaultError::BaseVaultAuthorityBumpIncorrect);
        }

        if self.token_mint == Pubkey::default() {
            return err!(KaminoVaultError::TokenMintIncorrect);
        }

        if self.token_mint_decimals == 0 {
            return err!(KaminoVaultError::TokenMintDecimalsIncorrect);
        }

        if self.token_vault == Pubkey::default() {
            return err!(KaminoVaultError::TokenVaultIncorrect);
        }

        if self.shares_mint == Pubkey::default() {
            return err!(KaminoVaultError::SharesMintIncorrect);
        }

        if self.shares_mint_decimals == 0 {
            return err!(KaminoVaultError::SharesMintDecimalsIncorrect);
        }

        if self.token_available != 0
            || self.shares_issued != 0
            || self.performance_fee_bps != 0
            || self.management_fee_bps != 0
            || self.pending_fees_sf != 0
            || self.last_fee_charge_timestamp != 0
            || self.prev_aum_sf != 0
        {
            return err!(KaminoVaultError::InitialAccountingIncorrect);
        }

        Ok(())
    }

    pub fn is_allocated_to_reserve(&self, reserve: Pubkey) -> bool {
       
        self.vault_allocation_strategy
            .iter()
            .any(|r| r.reserve == reserve)
    }

    pub fn allocation_for_reserve(&self, reserve: &Pubkey) -> Result<&VaultAllocation> {
        let allocation = self
            .vault_allocation_strategy
            .iter()
            .find(|a| a.reserve == *reserve)
            .ok_or_else(|| error!(KaminoVaultError::ReserveNotPartOfAllocations))?;

        Ok(allocation)
    }

    pub fn get_reserve_idx_in_allocation(&self, reserve: &Pubkey) -> Option<usize> {
        self.vault_allocation_strategy
            .iter()
            .position(|r| r.reserve.eq(reserve))
    }

    pub fn get_reserve_allocation_mut(&mut self, idx: usize) -> Result<&mut VaultAllocation> {
        self.vault_allocation_strategy
            .get_mut(idx)
            .ok_or(error!(KaminoVaultError::OutOfRangeOfReserveIndex))
    }

    pub fn upsert_reserve_allocation(
        &mut self,
        reserve: Pubkey,
        ctoken_vault: Pubkey,
        ctoken_vault_bump: u64,
        target_allocation_weight: u64,
        allocation_cap: u64,
        ctoken_allocation_cap: CtokenCap,
    ) -> Result<()> {
        let idx = self.get_reserve_idx_in_allocation(&reserve);

        match idx {
            Some(idx) => {
               
                self.vault_allocation_strategy[idx].target_allocation_weight =
                    target_allocation_weight;

                self.vault_allocation_strategy[idx].token_allocation_cap = allocation_cap;

                self.vault_allocation_strategy[idx].ctoken_allocation_cap =
                    ctoken_allocation_cap.raw();
            }
            None => {
               
                let idx = self
                    .vault_allocation_strategy
                    .iter()
                    .position(|r| {
                       
                        r.reserve == Pubkey::default()
                    })
                    .ok_or(error!(KaminoVaultError::ReserveSpaceExhausted))?;

                self.vault_allocation_strategy[idx] = VaultAllocation {
                    reserve,
                    ctoken_vault,
                    target_allocation_weight,
                    ctoken_allocation: 0,
                    token_target_allocation_sf: 0,
                    token_allocation_cap: allocation_cap,
                    ctoken_allocation_cap: ctoken_allocation_cap.raw(),
                    last_invest_slot: 0,
                    ctoken_vault_bump,
                    config_padding: [0; 126],
                    state_padding: [0; 128],
                };
            }
        }

        Ok(())
    }

    pub fn remove_reserve_from_allocation(&mut self, reserve: &Pubkey) -> Result<()> {
        let idx = self.get_reserve_idx_in_allocation(reserve);

        match idx {
            Some(idx) => {
                if self.vault_allocation_strategy[idx].can_be_removed() {
                    self.vault_allocation_strategy[idx] = Default::default();
                    Ok(())
                } else {
                    Err(error!(
                        KaminoVaultError::ReserveHasNonZeroAllocationOrCTokens
                    ))
                }
            }
            None => err!(KaminoVaultError::ReserveNotPartOfAllocations),
        }
    }

    pub fn refresh_target_allocations(&mut self, invested: &Invested) -> Result<()> {
        let total_tokens = self.compute_aum(&invested.total)?;
        let total_weight = self
            .vault_allocation_strategy
            .iter()
            .filter(|r| r.reserve != Pubkey::default() && r.token_allocation_cap > 0)
            .map(|r| r.target_allocation_weight)
            .sum::<u64>();

        let mut remaining_tokens_to_allocate = total_tokens;
        let mut token_target_allocations = [Fraction::ZERO; MAX_RESERVES];

       
        if self.unallocated_weight > 0 {
            let unallocated_cap = if self.unallocated_tokens_cap == 0 {
                u64::MAX
            } else {
                self.unallocated_tokens_cap
            };

            let unallocated_target = total_tokens.mul_int_ratio(
                self.unallocated_weight,
                total_weight + self.unallocated_weight,
            );
            let unallocated_tokens_target = unallocated_target.min(Fraction::from(unallocated_cap));
            remaining_tokens_to_allocate -= unallocated_tokens_target;
        }

        let mut remaining_weight_to_allocate = total_weight;

       
       
        while remaining_tokens_to_allocate > Fraction::ZERO && remaining_weight_to_allocate > 0 {
            let loop_total_tokens = remaining_tokens_to_allocate;
            let loop_weight = remaining_weight_to_allocate;
            let mut a_cap_was_reached = false;
            for ((allocation, invested_allocation), token_target_allocation) in self
                .vault_allocation_strategy
                .iter_mut()
                .zip(invested.allocations.iter())
                .zip(token_target_allocations.iter_mut())
                .filter(|((allocation, _), _)| allocation.reserve != Pubkey::default())
            {
                let reserve_weight = allocation.target_allocation_weight;
                let effective_token_allocation_cap =
                    allocation.effective_token_allocation_cap(invested_allocation);

                if *token_target_allocation >= effective_token_allocation_cap {
                    continue;
                }

                let reserve_target_ideal =
                    loop_total_tokens.mul_int_ratio(reserve_weight, loop_weight);

                let reserve_target_capped = if (reserve_target_ideal + *token_target_allocation)
                    >= effective_token_allocation_cap
                {
                    a_cap_was_reached = true;
                   
                    remaining_weight_to_allocate -= reserve_weight;
                    effective_token_allocation_cap - *token_target_allocation
                } else {
                    reserve_target_ideal
                };

                remaining_tokens_to_allocate -= reserve_target_capped;
                *token_target_allocation += reserve_target_capped;
            }
            if !a_cap_was_reached {
               
                break;
            }
        }

       
        for ((allocation, invested_allocation), token_target_allocation) in self
            .vault_allocation_strategy
            .iter_mut()
            .zip(invested.allocations.iter())
            .zip(token_target_allocations.iter())
            .filter(|((allocation, _), _)| allocation.reserve != Pubkey::default())
        {
            let effective_token_allocation_cap =
                allocation.effective_token_allocation_cap(invested_allocation);
            allocation.set_token_target_allocation(*token_target_allocation);

           
            const LOG_STRING_LENGTH: usize = 30 + 46 + 10 + 10 + 20 + 20 + 50;
            if *token_target_allocation < effective_token_allocation_cap {
                crate::kmsg_sized!(
                    LOG_STRING_LENGTH,
                    "Reserve {}: {}/{} target {} of total {}",
                    allocation.reserve,
                    allocation.target_allocation_weight,
                    total_weight,
                    token_target_allocation.to_floor::<u64>(),
                    total_tokens.to_floor::<u64>()
                );
            } else {
                crate::kmsg_sized!(
                    LOG_STRING_LENGTH,
                    "Reached allocation cap! Reserve {}: {}/{} target cap {} of total {}",
                    allocation.reserve,
                    allocation.target_allocation_weight,
                    total_weight,
                    effective_token_allocation_cap.to_floor::<u64>(),
                    total_tokens.to_floor::<u64>()
                );
            }
        }

        Ok(())
    }

    pub fn set_allocation_last_invest_slot(&mut self, reserve: &Pubkey, slot: u64) -> Result<()> {
        let idx = self.get_reserve_idx_in_allocation(reserve);

        match idx {
            Some(idx) => {
                self.vault_allocation_strategy[idx].set_last_invest_slot(slot);
                Ok(())
            }
            None => err!(KaminoVaultError::ReserveNotPartOfAllocations),
        }
    }
}

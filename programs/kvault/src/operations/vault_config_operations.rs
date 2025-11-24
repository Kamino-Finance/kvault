use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use kamino_lending::utils::FULL_BPS;

use crate::{
    operations::vault_operations::string_utils::{encoded_name_to_label, slice_to_array_padded},
    utils::consts::{
        MAX_MGMT_FEE_BPS, MAX_WITHDRAWAL_PENALTY_BPS, MAX_WITHDRAWAL_PENALTY_LAMPORTS,
        UPPER_LIMIT_MIN_WITHDRAW_AMOUNT,
    },
    KaminoVaultError::{self, BPSValueTooBig},
    VaultState,
};

#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum VaultConfigField {
    PerformanceFeeBps,
    ManagementFeeBps,
    MinDepositAmount,
    MinWithdrawAmount,
    MinInvestAmount,
    MinInvestDelaySlots,
    CrankFundFeePerReserve,
    PendingVaultAdmin,
    Name,
    LookupTable,
    Farm,
    AllocationAdmin,
    UnallocatedWeight,
    UnallocatedTokensCap,
    WithdrawalPenaltyLamports,
    WithdrawalPenaltyBps,
    FirstLossCapitalFarm,
    AllowAllocationsInWhitelistedReservesOnly,
    AllowInvestInWhitelistedReservesOnly,
}

pub fn check_if_signer_allowed_to_update_vault_config(
    entry: &VaultConfigField,
    data: &[u8],
    is_global_admin: bool,
    is_vault_admin: bool,
) -> Result<()> {
    match entry {
        VaultConfigField::AllowAllocationsInWhitelistedReservesOnly
        | VaultConfigField::AllowInvestInWhitelistedReservesOnly => {
            let value: u8 = BorshDeserialize::try_from_slice(data)?;
            // Both admins can set to true (1), only global admin can set to false (0)
            if value == 0 {
                require!(is_global_admin, KaminoVaultError::AdminAuthorityIncorrect);
            } else if value == 1 {
                require!(
                    is_global_admin || is_vault_admin,
                    KaminoVaultError::AdminAuthorityIncorrect
                );
            } else {
                return Err(KaminoVaultError::InvalidBoolLikeValue.into());
            }
        }
        VaultConfigField::PerformanceFeeBps
        | VaultConfigField::ManagementFeeBps
        | VaultConfigField::MinDepositAmount
        | VaultConfigField::MinWithdrawAmount
        | VaultConfigField::MinInvestAmount
        | VaultConfigField::MinInvestDelaySlots
        | VaultConfigField::CrankFundFeePerReserve
        | VaultConfigField::PendingVaultAdmin
        | VaultConfigField::Name
        | VaultConfigField::LookupTable
        | VaultConfigField::Farm
        | VaultConfigField::FirstLossCapitalFarm
        | VaultConfigField::AllocationAdmin
        | VaultConfigField::UnallocatedWeight
        | VaultConfigField::UnallocatedTokensCap
        | VaultConfigField::WithdrawalPenaltyLamports
        | VaultConfigField::WithdrawalPenaltyBps => {
            // For all other fields, only vault admin is allowed
            require!(is_vault_admin, KaminoVaultError::AdminAuthorityIncorrect);
        }
    }
    Ok(())
}

pub fn update_vault_config(
    vault: &mut VaultState,
    entry: VaultConfigField,
    data: &[u8],
) -> Result<()> {
    msg!("Updating vault config field {:?}", entry);
    match entry {
        VaultConfigField::PerformanceFeeBps => {
            let performance_fee_bps = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.performance_fee_bps);
            msg!("New value is {:?}", performance_fee_bps);
            let full_bps_u64: u64 = FULL_BPS.into();
            if performance_fee_bps > full_bps_u64 {
                return Err(BPSValueTooBig.into());
            }
            vault.performance_fee_bps = performance_fee_bps;
        }
        VaultConfigField::ManagementFeeBps => {
            let management_fee_bps = BorshDeserialize::try_from_slice(data)?;
            if management_fee_bps > MAX_MGMT_FEE_BPS {
                return err!(KaminoVaultError::ManagementFeeGreaterThanMaxAllowed);
            }

            msg!("Prv value is {:?}", vault.management_fee_bps);
            msg!("New value is {:?}", management_fee_bps);
            vault.management_fee_bps = management_fee_bps;
        }
        VaultConfigField::MinDepositAmount => {
            let min_deposit_amount = BorshDeserialize::try_from_slice(data)?;
            msg!("Prv value is {:?}", vault.min_deposit_amount);
            msg!("New value is {:?}", min_deposit_amount);
            vault.min_deposit_amount = min_deposit_amount;
        }
        VaultConfigField::MinWithdrawAmount => {
            let min_withdraw_amount = BorshDeserialize::try_from_slice(data)?;
            require!(
                min_withdraw_amount <= UPPER_LIMIT_MIN_WITHDRAW_AMOUNT,
                KaminoVaultError::MinWithdrawAmountTooBig
            );

            msg!("Prv value is {:?}", vault.min_withdraw_amount);
            msg!("New value is {:?}", min_withdraw_amount);
            vault.min_withdraw_amount = min_withdraw_amount;
        }
        VaultConfigField::MinInvestAmount => {
            let min_invest_amount = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.min_invest_amount);
            msg!("New value is {:?}", min_invest_amount);
            vault.min_invest_amount = min_invest_amount;
        }
        VaultConfigField::MinInvestDelaySlots => {
            let min_invest_delay_slots = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.min_invest_delay_slots);
            msg!("New value is {:?}", min_invest_delay_slots);
            vault.min_invest_delay_slots = min_invest_delay_slots;
        }
        VaultConfigField::CrankFundFeePerReserve => {
            let crank_fund_fee_per_reserve = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.crank_fund_fee_per_reserve);
            msg!("New value is {:?}", crank_fund_fee_per_reserve);
            vault.crank_fund_fee_per_reserve = crank_fund_fee_per_reserve;
        }
        VaultConfigField::PendingVaultAdmin => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.pending_admin);
            msg!("New value is {:?}", pubkey);
            vault.pending_admin = pubkey;
        }
        VaultConfigField::Name => {
            let str_name = encoded_name_to_label(data, vault.token_mint);

            msg!(
                "Prv value is {:?}",
                encoded_name_to_label(&vault.name, vault.token_mint)
            );
            msg!("New value is {:?}", str_name);
            let name = slice_to_array_padded(data);
            vault.name = name;
        }
        VaultConfigField::LookupTable => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.vault_lookup_table);
            msg!("New value is {:?}", pubkey);
            vault.vault_lookup_table = pubkey;
        }
        VaultConfigField::Farm => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.vault_farm);
            msg!("New value is {:?}", pubkey);
            vault.vault_farm = pubkey;
        }
        VaultConfigField::FirstLossCapitalFarm => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.first_loss_capital_farm);
            msg!("New value is {:?}", pubkey);
            vault.first_loss_capital_farm = pubkey;
        }
        VaultConfigField::AllocationAdmin => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.allocation_admin);
            msg!("New value is {:?}", pubkey);
            vault.allocation_admin = pubkey;
        }
        VaultConfigField::UnallocatedWeight => {
            let unallocated_weight = BorshDeserialize::try_from_slice(data)?;
            msg!("Prv value is {:?}", vault.unallocated_weight);
            msg!("New value is {:?}", unallocated_weight);
            vault.unallocated_weight = unallocated_weight;
        }
        VaultConfigField::UnallocatedTokensCap => {
            let unallocated_tokens_cap = BorshDeserialize::try_from_slice(data)?;
            msg!("Prv value is {:?}", vault.unallocated_tokens_cap);
            msg!("New value is {:?}", unallocated_tokens_cap);
            vault.unallocated_tokens_cap = unallocated_tokens_cap;
        }
        VaultConfigField::WithdrawalPenaltyLamports => {
            let withdrawal_penalty_lamports = BorshDeserialize::try_from_slice(data)?;
            require_gte!(
                MAX_WITHDRAWAL_PENALTY_LAMPORTS,
                withdrawal_penalty_lamports,
                KaminoVaultError::WithdrawalFeeLamportsGreaterThanMaxAllowed
            );
            msg!("Prv value is {:?}", vault.withdrawal_penalty_lamports);
            msg!("New value is {:?}", withdrawal_penalty_lamports);
            vault.withdrawal_penalty_lamports = withdrawal_penalty_lamports;
        }
        VaultConfigField::WithdrawalPenaltyBps => {
            let withdrawal_penalty_bps = BorshDeserialize::try_from_slice(data)?;
            require_gte!(
                MAX_WITHDRAWAL_PENALTY_BPS,
                withdrawal_penalty_bps,
                KaminoVaultError::WithdrawalFeeBPSGreaterThanMaxAllowed
            );
            msg!("Prv value is {:?}", vault.withdrawal_penalty_bps);
            msg!("New value is {:?}", withdrawal_penalty_bps);
            vault.withdrawal_penalty_bps = withdrawal_penalty_bps;
        }
        VaultConfigField::AllowAllocationsInWhitelistedReservesOnly => {
            let value: u8 = BorshDeserialize::try_from_slice(data)?;
            msg!(
                "Prv value is {:?}",
                vault.allow_allocations_in_whitelisted_reserves_only
            );
            msg!("New value is {:?}", value);
            vault.allow_allocations_in_whitelisted_reserves_only = value;
        }
        VaultConfigField::AllowInvestInWhitelistedReservesOnly => {
            let value: u8 = BorshDeserialize::try_from_slice(data)?;
            msg!(
                "Prv value is {:?}",
                vault.allow_invest_in_whitelisted_reserves_only
            );
            msg!("New value is {:?}", value);
            vault.allow_invest_in_whitelisted_reserves_only = value;
        }
    }

    Ok(())
}

use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use kamino_lending::{
    utils::{FatAccountLoader, FULL_BPS},
    Reserve,
};

use crate::{
    operations::{
        klend_operations,
        vault_operations::{
            self,
            common::holdings,
            string_utils::{encoded_name_to_label, slice_to_array_padded},
        },
    },
    utils::{
        consts::{MAX_MGMT_FEE_BPS, UPPER_LIMIT_MIN_WITHDRAW_AMOUNT},
        cpi_mem::CpiMemoryLender,
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
}

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateVaultConfig<'info>>,
    entry: VaultConfigField,
    data: &[u8],
) -> Result<()> {
    // CPI memory allocation
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    let reserves_count = vault.get_reserves_count();
    {
        // Refresh all reserves
        klend_operations::cpi_refresh_reserves(
            &mut cpi_mem,
            ctx.remaining_accounts.iter().take(reserves_count),
            reserves_count,
        )?;
    }
    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    let holdings = holdings(vault, reserves_iter, Clock::get()?.slot)?;
    msg!("holdings {:?}", holdings);
    // charge fees because after this the fee structure can be different
    vault_operations::charge_fees(
        vault,
        &holdings.invested,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;

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
        VaultConfigField::AllocationAdmin => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;

            msg!("Prv value is {:?}", vault.allocation_admin);
            msg!("New value is {:?}", pubkey);
            vault.allocation_admin = pubkey;
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateVaultConfig<'info> {
    pub vault_admin_authority: Signer<'info>,

    #[account(mut,
        has_one = vault_admin_authority,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

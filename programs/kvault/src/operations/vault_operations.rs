use core::fmt;
use std::{fmt::Debug, ops::Mul};

use anchor_lang::{err, prelude::*, require, solana_program::clock::Slot, Result};
use common::{compute_user_total_received_on_withdraw, update_prev_aum, Holdings, Invested};
use kamino_lending::{
    fraction::Fraction,
    utils::{AnyAccountLoader, FractionExtra},
    Reserve,
};
use rust_decimal::prelude::ToPrimitive;
use solana_program::pubkey::Pubkey;

use super::effects::{
    DepositEffects, InvestEffects, InvestingDirection, WithdrawEffects, WithdrawPendingFeesEffects,
};
use crate::{
    kmsg, kmsg_sized,
    operations::vault_operations::common::{get_shares_to_mint, holdings},
    utils::consts::SECONDS_PER_YEAR,
    xmsg, KaminoVaultError, VaultState, MAX_RESERVES,
};

pub fn initialize(
    vault: &mut VaultState,
    token_decimals: u8,
    shares_decimals: u8,
    current_timestamp: u64,
) -> Result<()> {
    xmsg!(
        "Initializing with token_decimals={} shares_decimals={}",
        token_decimals,
        shares_decimals
    );

    vault.token_mint_decimals = token_decimals as u64;
    vault.shares_mint_decimals = shares_decimals as u64;
    vault.token_available = 0;
    vault.shares_issued = 0;
    vault.creation_timestamp = current_timestamp;

    vault.validate()
}

#[inline(never)]
pub fn deposit<'info, T>(
    vault: &mut VaultState,
    reserves_iter: impl Iterator<Item = T>,
    max_amount: u64,
    current_slot: Slot,
    current_timestamp: u64,
) -> Result<DepositEffects>
where
    T: AnyAccountLoader<'info, Reserve>,
{
    let num_reserve: u64 = vault
        .get_reserves_with_allocation_count()
        .try_into()
        .unwrap();
    let crank_funds_to_deposit = num_reserve * vault.crank_fund_fee_per_reserve;

    let max_user_tokens_to_deposit = max_amount - crank_funds_to_deposit;

    let holdings = holdings(vault, reserves_iter, current_slot)?;

    kmsg!(
        "holdings available {} total invested {}",
        holdings.available,
        holdings.invested.total
    );
    kmsg!("shares_issued before deposit {}", vault.shares_issued);

    charge_fees(vault, &holdings.invested, current_timestamp)?;
    let current_vault_aum = vault.compute_aum(&holdings.invested.total)?;

    let shares_to_mint = get_shares_to_mint(
        current_vault_aum,
        max_user_tokens_to_deposit,
        vault.shares_issued,
    )?;
    let user_tokens_to_deposit = common::compute_amount_to_deposit_from_shares_to_mint(
        vault.shares_issued,
        current_vault_aum,
        shares_to_mint,
    );

    if user_tokens_to_deposit < vault.min_deposit_amount {
        return err!(KaminoVaultError::DepositAmountBelowMinimum);
    }

    if shares_to_mint == 0 {
        return err!(KaminoVaultError::DepositAmountsZeroShares);
    }

    // EFFECTS: These are always the last things to update, their order matters for fee tracking
    common::deposit_into_vault(vault, user_tokens_to_deposit);
    common::mint_shares(vault, shares_to_mint);
    common::update_prev_aum(
        vault,
        current_vault_aum + Fraction::from(user_tokens_to_deposit),
    );
    common::deposit_crank_funds(vault, crank_funds_to_deposit);

    Ok(DepositEffects {
        shares_to_mint,
        token_to_deposit: user_tokens_to_deposit,
        crank_funds_to_deposit,
    })
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn withdraw<'info, T>(
    vault: &mut VaultState,
    reserve_address_to_withdraw_from: Option<&Pubkey>,
    reserve_state_to_withdraw_from: Option<&Reserve>,
    reserves_iter: impl Iterator<Item = T>,
    current_timestamp: u64,
    current_slot: Slot,
    number_of_shares: u64,
    reserve_ctokens_owned: Option<u64>,
) -> Result<WithdrawEffects>
where
    T: AnyAccountLoader<'info, Reserve>,
{
    require!(
        number_of_shares > 0,
        KaminoVaultError::CannotWithdrawZeroShares
    );

    // Get total amounts
    let holdings = holdings(vault, reserves_iter, current_slot)?;

    charge_fees(vault, &holdings.invested, current_timestamp)?;

    // user is entitled to his corresponding ratio of the total - pending fees
    let current_vault_aum = vault.compute_aum(&holdings.invested.total)?;

    require!(
        current_vault_aum > Fraction::ZERO,
        KaminoVaultError::VaultAUMZero
    );

    // How much the user has to receive (rounded down to the nearest integer, so if the user is entitled t0 10.12 lamports it will return 10)
    let total_shares_supply = vault.shares_issued;
    let total_for_user: u64 = compute_user_total_received_on_withdraw(
        total_shares_supply,
        current_vault_aum,
        number_of_shares,
    );

    // Calculate how much the user is allowed to withdraw given a max combo of
    // available + reserve
    let available_to_send_to_user = holdings.available.min(total_for_user); // First max out from available

    let (
        invested_liquidity_to_send_to_user_f,
        invested_liquidity_to_disinvest,
        invested_to_disinvest_ctokens,
        liquidity_rounding_error,
    ) = if let Some(reserve_address) = reserve_address_to_withdraw_from {
        let invested_in_reserve = holdings.invested.in_reserve(reserve_address);
        // get the needed liquidity to withdraw from invested or the whole invested if this is not enough

        let invested_liquidity_to_send_to_user_f = invested_in_reserve
            .liquidity_amount
            .min(Fraction::from(total_for_user - available_to_send_to_user)); // Then keep drawing from invested in this current reserve

        // Early return if the available is enough to send to the user
        if invested_liquidity_to_send_to_user_f.eq(&Fraction::ZERO) {
            (Fraction::ZERO, 0, 0, 0)
        } else {
            let exchange_rate = reserve_state_to_withdraw_from
                .unwrap()
                .collateral_exchange_rate();

            // round up the cTokens to burn so we have enough liquidity to send to the user
            let invested_to_disinvest_ctokens: u64 = exchange_rate
                .fraction_liquidity_to_collateral_ceil(invested_liquidity_to_send_to_user_f.floor())
                .to_ceil();
            let max_ctokens_to_disinvest = reserve_ctokens_owned.unwrap_or(0);
            // cap the cTokens to burn to the max the allocation has
            let invested_to_disinvest_ctokens =
                invested_to_disinvest_ctokens.min(max_ctokens_to_disinvest);

            // compute the expected total liquidity disinvested as we can withdraw slightly more than the user is entitled to so we make sure we have enough for the withdrawal
            let invested_liquidity_to_disinvest_f = exchange_rate.fraction_collateral_to_liquidity(
                Fraction::from_num(invested_to_disinvest_ctokens),
            );
            let invested_liquidity_to_disinvest =
                invested_liquidity_to_disinvest_f.to_floor::<u64>();

            // when we burn ctokens we only get the integer part of the liquidity so the vault may lose a bit of liquidity (e.g. if the ctokens represent 10.12 lamports, we will only get 10). If the fractional part that is lost is greater than the fractional part of the user's withdrawal it means that the vault loses on this withdrawal and we need to take 1 lamport from the user received amount as a compensation for the loss
            let liquidity_rounding_error: u64 = if invested_liquidity_to_disinvest_f.frac()
                > Fraction::ZERO
                && invested_liquidity_to_disinvest_f.frac()
                    > invested_liquidity_to_send_to_user_f.frac()
            {
                1
            } else {
                0
            };
            (
                invested_liquidity_to_send_to_user_f,
                invested_liquidity_to_disinvest,
                invested_to_disinvest_ctokens,
                liquidity_rounding_error,
            )
        }
    } else {
        (Fraction::ZERO, 0, 0, 0)
    };

    let invested_liquidity_to_send_to_user: u64 = invested_liquidity_to_send_to_user_f.to_floor();
    let theoretical_amount_to_send_to_user_f =
        Fraction::from(available_to_send_to_user) + invested_liquidity_to_send_to_user_f;
    let actual_invested_liquidity_to_send_to_user =
        invested_liquidity_to_send_to_user - liquidity_rounding_error;

    let shares_to_burn = common::calculate_shares_to_burn(
        theoretical_amount_to_send_to_user_f,
        total_shares_supply,
        current_vault_aum,
        number_of_shares,
    );

    let disinvested_amount_left_in_vault =
        invested_liquidity_to_disinvest - actual_invested_liquidity_to_send_to_user;

    // if the withdraw represents 0 shares fail the tx
    if shares_to_burn == 0 {
        return err!(KaminoVaultError::WithdrawResultsInZeroShares);
    }

    kmsg!("Available {}", holdings.available);
    kmsg!("Total invested {:?}", holdings.invested.total.to_display());
    kmsg!("Available to send to user {}", available_to_send_to_user);
    kmsg!("Shares to burn {}", shares_to_burn);
    kmsg!("Disinvest liq {}", invested_liquidity_to_send_to_user);
    kmsg!(
        "Actual invested liq {}",
        actual_invested_liquidity_to_send_to_user
    );
    kmsg!("Expected c tokens {}", invested_to_disinvest_ctokens);
    kmsg!("Expected liq {}", invested_liquidity_to_disinvest);

    if available_to_send_to_user + invested_liquidity_to_send_to_user <= vault.min_withdraw_amount {
        return err!(KaminoVaultError::WithdrawAmountBelowMinimum);
    }

    if let Some(reserve_address) = reserve_address_to_withdraw_from {
        if !vault.is_allocated_to_reserve(*reserve_address) {
            return err!(KaminoVaultError::ReserveNotPartOfAllocations);
        }
    }

    // EFFECTS: Accounting
    common::withdraw_from_accounting(vault, available_to_send_to_user, shares_to_burn);
    common::deposit_into_vault(vault, disinvested_amount_left_in_vault);
    if let Some(reserve_address) = reserve_address_to_withdraw_from {
        common::withdraw_from_vault_allocation(
            vault,
            invested_to_disinvest_ctokens,
            reserve_address,
        )?;
    }
    common::update_prev_aum(
        vault,
        current_vault_aum - theoretical_amount_to_send_to_user_f,
    );

    Ok(WithdrawEffects {
        shares_to_burn,
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user: actual_invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest,
    })
}

#[inline(never)]
pub fn withdraw_pending_fees<'info, T>(
    vault: &mut VaultState,
    reserve_address_to_withdraw_from: &Pubkey,
    reserve_state_to_withdraw_from: &Reserve,
    reserves_iter: impl Iterator<Item = T>,
    current_slot: Slot,
    current_timestamp: u64,
) -> Result<WithdrawPendingFeesEffects>
where
    T: AnyAccountLoader<'info, Reserve>,
{
    // Get total amounts
    let Holdings {
        invested,
        available,
        total_sum,
    } = holdings(vault, reserves_iter, current_slot)?;

    msg!(
        "holdings invested {:?} available {:?} total_sum {}",
        invested,
        available,
        total_sum.to_display()
    );

    charge_fees(vault, &invested, current_timestamp)?;

    let total_fees = Fraction::from_bits(vault.pending_fees_sf);

    // withdraw maximum possible from the available
    let available_to_send_to_user_f = Fraction::from(available).min(total_fees);
    let available_to_send_to_user = available_to_send_to_user_f.to_floor::<u64>();

    let invested_in_reserve = invested.in_reserve(reserve_address_to_withdraw_from);
    let invested_liquidity_to_send_to_user_f = invested_in_reserve
        .liquidity_amount
        .min(total_fees - available_to_send_to_user_f);
    let invested_liquidity_to_send_to_user: u64 = invested_liquidity_to_send_to_user_f.to_floor();

    let exchange_rate = reserve_state_to_withdraw_from.collateral_exchange_rate();

    let invested_to_disinvest_ctokens: u64 =
        exchange_rate.liquidity_to_collateral_ceil(invested_liquidity_to_send_to_user);

    // compute the expected total liquidity disinvested as we can withdraw slightly less than the user is entitled to so we make sure we have enough for the withdrawal
    let invested_liquidity_to_disinvest_f =
        exchange_rate.fraction_collateral_to_liquidity(invested_to_disinvest_ctokens.into());
    let invested_liquidity_to_disinvest = invested_liquidity_to_disinvest_f.to_floor::<u64>();

    let liquidity_rounding_error = if invested_liquidity_to_disinvest_f.frac() > Fraction::ZERO {
        1
    } else {
        0
    };

    let actual_invested_liquidity_to_send_to_user =
        invested_liquidity_to_send_to_user - liquidity_rounding_error;
    let disinvested_amount_left_in_vault =
        invested_liquidity_to_disinvest - actual_invested_liquidity_to_send_to_user;

    // Accounting
    common::withdraw_from_vault(vault, available_to_send_to_user);
    common::deposit_into_vault(vault, disinvested_amount_left_in_vault);
    common::withdraw_from_vault_allocation(
        vault,
        invested_to_disinvest_ctokens,
        reserve_address_to_withdraw_from,
    )?;

    common::update_pending_fees(
        vault,
        total_fees
            - Fraction::from(available_to_send_to_user)
            - Fraction::from(invested_liquidity_to_send_to_user),
    );

    Ok(WithdrawPendingFeesEffects {
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user: actual_invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest,
    })
}

pub fn give_up_pending_fee<'info, T>(
    vault: &mut VaultState,
    reserves_iter: impl Iterator<Item = T>,
    current_slot: Slot,
    current_timestamp: u64,
    max_amount_to_give_up: u64,
) -> Result<()>
where
    T: AnyAccountLoader<'info, Reserve>,
{
    let holdings = holdings(vault, reserves_iter, current_slot)?;
    msg!("holdings {:?}", holdings);
    let invested = &holdings.invested;

    charge_fees(vault, invested, current_timestamp)?;
    let amount = Fraction::from(max_amount_to_give_up);
    let pending_fees = vault.get_pending_fees();
    let amount_to_give_up = amount.min(pending_fees);

    msg!(
        "Giving up {} of {} pending fees",
        amount_to_give_up.to_display(),
        pending_fees.to_display()
    );

    let new_pending_fees = pending_fees - amount_to_give_up;

    common::update_pending_fees(vault, new_pending_fees);

    // give up pending fee is aimed at unlocking a vault that suffered a loss.
    // To ensure the next charge fee don't prevent this, update last_fee_charge_timestamp to the current timestamp and increase the prev aum by the amount given up

    vault.last_fee_charge_timestamp = current_timestamp;
    let prev_aum = holdings.total_sum.saturating_sub(new_pending_fees);
    msg!("holdings.total_sum {}", holdings.total_sum.to_display());
    msg!("new_pending_fees {}", new_pending_fees.to_display());
    msg!("prev_aum {}", prev_aum.to_display());
    common::update_prev_aum(vault, prev_aum);

    Ok(())
}

#[inline(never)]
pub fn invest<'info, T>(
    vault: &mut VaultState,
    reserves_iter: impl Iterator<Item = T>,
    reserve: &Reserve,
    reserve_address: &Pubkey,
    current_slot: Slot,
    current_timestamp: u64,
) -> Result<InvestEffects>
where
    T: AnyAccountLoader<'info, Reserve>,
{
    let holdings = holdings(vault, reserves_iter, current_slot)?;
    kmsg_sized!(50, "holdings available {}", holdings.available);
    kmsg_sized!(
        50,
        "holdings invested {}",
        holdings.invested.total.to_display()
    );
    let invested = holdings.invested;

    charge_fees(vault, &invested, current_timestamp)?;

    vault.refresh_target_allocations(&invested)?;

    if !vault.is_allocated_to_reserve(*reserve_address) {
        return err!(KaminoVaultError::ReserveNotPartOfAllocations);
    }

    let allocation_for_reserve = vault.allocation_for_reserve(reserve_address)?;
    kmsg!(
        "alloc_for_reserve address {} weight {} cap {}",
        allocation_for_reserve.reserve,
        allocation_for_reserve.target_allocation_weight,
        allocation_for_reserve.token_allocation_cap
    );

    if current_slot < allocation_for_reserve.last_invest_slot + vault.min_invest_delay_slots {
        return err!(KaminoVaultError::InvestTooSoon);
    }

    let invested_in_reserve = invested.in_reserve(reserve_address);

    let actual_tokens_invested = invested_in_reserve.liquidity_amount;
    let target_tokens_invested = allocation_for_reserve.get_token_target_allocation();
    let (liquidity_f, direction) = if actual_tokens_invested > target_tokens_invested {
        let diff = actual_tokens_invested - target_tokens_invested;
        kmsg!(
            "Actual {} target {}, need to Subtract {}",
            actual_tokens_invested.to_display(),
            target_tokens_invested.to_display(),
            diff.to_display()
        );

        (diff, InvestingDirection::Subtract)
    } else {
        let diff = target_tokens_invested - actual_tokens_invested;
        let available = common::available_to_invest(vault);
        kmsg!(
            "Actual {} target {} available {}, need to Add {}",
            actual_tokens_invested.to_display(),
            target_tokens_invested.to_display(),
            available,
            diff.to_display()
        );

        (diff.min(Fraction::from(available)), InvestingDirection::Add)
    };

    if liquidity_f <= vault.min_invest_amount {
        return err!(KaminoVaultError::InvestAmountBelowMinimum);
    }

    let exchange_rate = reserve.collateral_exchange_rate();
    let collateral_amount = if allocation_for_reserve.target_allocation_weight == 0 {
        allocation_for_reserve.ctoken_allocation
    } else {
        let collateral_f = exchange_rate.fraction_liquidity_to_collateral(liquidity_f);
        collateral_f.to_floor()
    };

    // Recompute liquidity amount from the collateral amount to know how much we will really receive/or need to deposit
    let liquidity_amount_f =
        exchange_rate.fraction_collateral_to_liquidity(collateral_amount.into());
    let liquidity_amount: u64;
    let mut rounding_loss: u64 = if liquidity_amount_f.frac() > Fraction::ZERO {
        1
    } else {
        0
    };

    match direction {
        InvestingDirection::Add => {
            // The minimum liquidity needed to get the desired collateral amount
            liquidity_amount = liquidity_amount_f.to_ceil();
            common::withdraw_from_vault(vault, liquidity_amount - rounding_loss);
            common::deposit_into_vault_allocation(vault, collateral_amount, reserve_address)?;
        }
        InvestingDirection::Subtract => {
            // The liquidity that will be received for the withdrawn collateral amount
            liquidity_amount = liquidity_amount_f.to_floor();
            common::deposit_into_vault(vault, liquidity_amount + rounding_loss);
            common::withdraw_from_vault_allocation(vault, collateral_amount, reserve_address)?;
        }
    }

    // Rounding loss is returned to the caller to be compensated only if there
    // is not enough fund available to cover it
    if vault.available_crank_funds >= rounding_loss {
        vault.available_crank_funds -= rounding_loss;
        rounding_loss = 0;
    }

    vault.set_allocation_last_invest_slot(reserve_address, current_slot)?;
    Ok(InvestEffects {
        liquidity_amount,
        direction,
        collateral_amount,
        rounding_loss,
    })
}

pub fn charge_fees(vault: &mut VaultState, invested: &Invested, timestamp: u64) -> Result<()> {
    if vault.last_fee_charge_timestamp == 0 {
        vault.last_fee_charge_timestamp = timestamp;
        return Ok(());
    }

    let seconds_passed = timestamp.saturating_sub(vault.last_fee_charge_timestamp);

    let new_aum = vault.compute_aum(&invested.total).unwrap_or(Fraction::ZERO);
    let prev_aum = vault.get_prev_aum();

    // Use our new kmsg! macro which is cleaner and more efficient
    crate::kmsg_sized!(
        300,
        "prev_aum {} new_aum {} seconds_passed {}",
        prev_aum.to_display(),
        new_aum.to_display(),
        seconds_passed
    );

    // if 0 seconds passed don't compute mgmt_fee
    let mgmt_charge = if seconds_passed == 0 {
        Fraction::ZERO
    } else {
        // Mgmt fee is applied to prev AUM
        let mgmt_fee_yearly = Fraction::from_bps(vault.management_fee_bps);
        let mgmt_fee = mgmt_fee_yearly * u128::from(seconds_passed)
            / SECONDS_PER_YEAR.ceil().to_u128().unwrap();
        let mgmt_charge = Fraction::from(prev_aum).mul(mgmt_fee);

        crate::kmsg_sized!(
            250,
            "mgmt_charge {} mgmt_fee {}",
            mgmt_charge.to_display(),
            mgmt_fee.to_display()
        );

        mgmt_charge
    };

    // Performance fee is applied to the interest earned; if there was a loss we don't charge any performance fee
    let earned_interest = new_aum.saturating_sub(prev_aum);
    let perf_charge = Fraction::from_bps(vault.performance_fee_bps) * earned_interest;

    crate::kmsg_sized!(
        250,
        "perf_charge {} earned_interest {}",
        perf_charge.to_display(),
        earned_interest.to_display()
    );

    vault.set_cumulative_mgmt_fees(vault.get_cumulative_mgmt_fees().saturating_add(mgmt_charge));
    vault.set_cumulative_perf_fees(vault.get_cumulative_perf_fees().saturating_add(perf_charge));
    vault.set_cumulative_earned_interest(
        vault
            .get_cumulative_earned_interest()
            .saturating_add(earned_interest),
    );

    let new_fees = (mgmt_charge + perf_charge).min(new_aum);
    let pending_fees = vault.get_pending_fees() + new_fees;
    vault.set_pending_fees(pending_fees);
    update_prev_aum(vault, new_aum - new_fees);
    vault.last_fee_charge_timestamp = timestamp;

    Ok(())
}

pub mod common {
    use anchor_lang::{error, Result};
    use kamino_lending::{utils::AnyAccountLoader, PriceStatusFlags, Reserve};
    use solana_program::pubkey::Pubkey;

    use super::*;

    pub fn get_shares_to_mint(
        holdings_aum: Fraction,
        user_token_amount: u64,
        shares_issued: u64,
    ) -> Result<u64> {
        if shares_issued == 0 {
            return Ok(user_token_amount);
        }

        if shares_issued != 0 && holdings_aum == Fraction::ZERO {
            return err!(KaminoVaultError::VaultAUMZero);
        }

        let shares_to_mint = Fraction::from(shares_issued)
            .full_mul_int_ratio(user_token_amount, holdings_aum.to_ceil::<u64>());

        Ok(shares_to_mint.to_floor())
    }

    pub fn amounts_invested<'info, T>(
        vault: &VaultState,
        mut reserves_iter: impl Iterator<Item = T>,
        slot: Slot,
    ) -> Result<Invested>
    where
        T: AnyAccountLoader<'info, Reserve>,
    {
        let mut invested = Invested::default();
        let mut total = Fraction::ZERO;

        for (allocation_state, computed_invested_allocation) in vault
            .vault_allocation_strategy
            .iter()
            .zip(invested.allocations.iter_mut())
        {
            if allocation_state.reserve == Pubkey::default() {
                // Skip empty allocations (leave result as default as a placeholder)
                continue;
            }

            let Some(reserve) = reserves_iter.next() else {
                return err!(KaminoVaultError::ReserveNotProvidedInTheAccounts);
            };
            let reserve_key = reserve.get_pubkey();

            let reserve = reserve
                .get()
                .map_err(|_| error!(KaminoVaultError::CouldNotDeserializeAccountAsReserve))?;

            if reserve_key != allocation_state.reserve {
                return err!(KaminoVaultError::ReserveAccountAndKeyMismatch);
            }

            if reserve
                .last_update
                .is_stale(slot, PriceStatusFlags::NONE)
                .unwrap()
            {
                return err!(KaminoVaultError::ReserveIsStale);
            }

            let ctoken_amount = allocation_state.ctoken_allocation;

            // Compute liquidity directly without temporary variables
            let liquidity_amount = reserve
                .collateral_exchange_rate()
                .fraction_collateral_to_liquidity(ctoken_amount.into());

            computed_invested_allocation.reserve = allocation_state.reserve;
            computed_invested_allocation.liquidity_amount = liquidity_amount;
            computed_invested_allocation.ctoken_amount = ctoken_amount;
            computed_invested_allocation.target_weight = allocation_state.target_allocation_weight;

            total += liquidity_amount;
        }

        invested.total = total;

        Ok(invested)
    }

    pub fn holdings<'info, T>(
        vault: &VaultState,
        reserves_iter: impl Iterator<Item = T>,
        slot: Slot,
    ) -> Result<Holdings>
    where
        T: AnyAccountLoader<'info, Reserve>,
    {
        let (available, invested) = underlying_inventory(vault, reserves_iter, slot)?;
        let total_sum = Fraction::from(available) + invested.total;

        Ok(Holdings {
            available,
            invested,
            total_sum,
        })
    }

    pub fn underlying_inventory<'info, T>(
        vault: &VaultState,
        reserves_iter: impl Iterator<Item = T>,
        slot: Slot,
    ) -> Result<(u64, Invested)>
    where
        T: AnyAccountLoader<'info, Reserve>,
    {
        let available = available_to_invest(vault);
        let invested = amounts_invested(vault, reserves_iter, slot)?;
        Ok((available, invested))
    }

    pub fn available_to_invest(vault: &VaultState) -> u64 {
        vault.token_available
    }

    pub fn deposit_into_vault(vault: &mut VaultState, amount: u64) {
        vault.token_available += amount;
    }

    pub fn withdraw_from_vault(vault: &mut VaultState, amount: u64) {
        vault.token_available -= amount;
    }

    pub fn compute_user_total_received_on_withdraw(
        shares_issued: u64,
        vault_total_holdings: Fraction, // this is the AUM - pending fees
        shares_to_withdraw: u64,
    ) -> u64 {
        let total_for_user: u64 = if shares_issued == shares_to_withdraw {
            vault_total_holdings.to_floor()
        } else {
            vault_total_holdings
                .full_mul_int_ratio(shares_to_withdraw, shares_issued)
                .to_floor()
        };
        kmsg_sized!(
            150,
            "Total for user {} total_sum {}",
            total_for_user,
            vault_total_holdings.to_display()
        );

        total_for_user
    }

    pub fn compute_amount_to_deposit_from_shares_to_mint(
        vault_total_shares: u64,
        vault_total_holdings: Fraction,
        shares_to_mint: u64,
    ) -> u64 {
        if vault_total_shares == 0 {
            shares_to_mint
        } else {
            vault_total_holdings
                .full_mul_int_ratio_ceil(shares_to_mint, vault_total_shares)
                .to_ceil()
        }
    }

    /// apply the withdrawal effects to the vault accounting
    pub fn withdraw_from_accounting(
        vault: &mut VaultState,
        available_to_send_to_user: u64,
        shares_to_burn: u64,
    ) {
        common::withdraw_from_vault(vault, available_to_send_to_user);
        common::burn_shares(vault, shares_to_burn);
    }

    pub fn calculate_shares_to_burn(
        amount_to_send_to_user: Fraction,
        total_supply: u64,
        total_sum: Fraction,
        number_of_shares: u64,
    ) -> u64 {
        (amount_to_send_to_user
            .full_mul_int_ratio(total_supply, total_sum.to_floor::<u64>())
            .to_ceil::<u64>())
        .min(number_of_shares)
    }

    pub fn deposit_into_vault_allocation(
        vault: &mut VaultState,
        ctokens: u64,
        reserve: &Pubkey,
    ) -> Result<()> {
        let idx = vault
            .get_reserve_idx_in_allocation(reserve)
            .ok_or(error!(KaminoVaultError::CannotFindReserveInAllocations))?;

        vault.get_reserve_allocation_mut(idx)?.ctoken_allocation += ctokens;

        Ok(())
    }

    pub fn withdraw_from_vault_allocation(
        vault: &mut VaultState,
        ctokens: u64,
        reserve: &Pubkey,
    ) -> Result<()> {
        let idx = vault
            .get_reserve_idx_in_allocation(reserve)
            .ok_or(error!(KaminoVaultError::CannotFindReserveInAllocations))?;

        vault.get_reserve_allocation_mut(idx)?.ctoken_allocation -= ctokens;

        Ok(())
    }

    pub fn burn_shares(vault: &mut VaultState, amt: u64) {
        vault.shares_issued -= amt;
    }

    pub fn mint_shares(vault: &mut VaultState, amt: u64) {
        vault.shares_issued += amt;
    }

    pub fn update_prev_aum(vault: &mut VaultState, aum: Fraction) {
        // This is for fee tracking
        vault.set_prev_aum(aum);
    }

    pub fn update_pending_fees(vault: &mut VaultState, fees: Fraction) {
        vault.set_pending_fees(fees);
    }

    pub fn deposit_crank_funds(vault: &mut VaultState, amount: u64) {
        vault.available_crank_funds += amount;
    }

    #[derive(Clone)]
    pub struct Holdings {
        pub available: u64,
        pub invested: Invested,
        pub total_sum: Fraction,
    }

    impl Debug for Holdings {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Holdings")
                .field("available", &self.available)
                .field("invested", &self.invested)
                .field("total_sum", &self.total_sum.to_display())
                .finish()
        }
    }

    #[derive(Default, Clone)]
    pub struct InvestedReserve {
        pub reserve: Pubkey,
        pub liquidity_amount: Fraction,
        pub ctoken_amount: u64,
        pub target_weight: u64,
    }

    impl fmt::Debug for InvestedReserve {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("InvestedReserve")
                .field("reserve", &self.reserve)
                .field("liquidity_amount", &self.liquidity_amount.to_display())
                .field("ctoken_amount", &self.ctoken_amount)
                .field("target_weight", &self.target_weight)
                .finish()
        }
    }

    #[derive(Default, Clone)]
    pub struct Invested {
        pub allocations: Box<[InvestedReserve; MAX_RESERVES]>,
        pub total: Fraction,
    }

    impl fmt::Debug for Invested {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let allocations_filtered: Vec<InvestedReserve> = self
                .allocations
                .iter()
                .filter(|i| i.reserve != Pubkey::default())
                .cloned()
                .collect();

            f.debug_struct("")
                .field("total", &self.total.to_display())
                .field("allocations", &allocations_filtered)
                .finish()
        }
    }

    impl Invested {
        pub fn in_reserve(&self, reserve: &Pubkey) -> &InvestedReserve {
            self.allocations
                .iter()
                .find(|a| a.reserve == *reserve)
                .ok_or(error!(KaminoVaultError::ReserveNotPartOfAllocations))
                .unwrap()
        }
    }
}

pub mod string_utils {
    use anchor_lang::prelude::Pubkey;
    pub fn encoded_name_to_label(encoded_name: &[u8], mint: Pubkey) -> String {
        std::str::from_utf8(encoded_name)
            .map(|x| x.trim_matches(char::from(0)).to_string())
            .unwrap_or_else(|_| format!("k{}", mint))
    }

    pub fn slice_to_array_padded(slice: &[u8]) -> [u8; 40] {
        // Initialize a new array filled with zeros
        let mut array = [0u8; 40];

        // Copy elements from the slice to the array
        let len = slice.len().min(40); // Ensure we don't go out of bounds
        array[..len].copy_from_slice(&slice[..len]);

        array
    }
}

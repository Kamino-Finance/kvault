use anchor_lang::prelude::*;
use fixed::types::U68F60 as Fraction;

use super::effects::{
    InvestEffects, InvestingDirection, WithdrawEffects, WithdrawPendingFeesEffects,
};
use crate::{require_msg, KaminoVaultError};

pub struct VaultAndUserBalances {
    pub reserve_supply_liquidity_balance: u64,
    pub vault_token_balance: u64,
    pub vault_ctoken_balance: u64,
    pub user_token_balance: u64,
    pub user_shares_balance: u64,
}

pub struct VaultBalances {
    pub reserve_supply_liquidity_balance: u64,
    pub vault_token_balance: u64,
    pub vault_ctoken_balance: u64,
}

pub fn post_transfer_withdraw_balance_checks(
    amounts_before: VaultAndUserBalances,
    amounts_after: VaultAndUserBalances,
    withdraw_effects: WithdrawEffects,
) -> Result<()> {
    let WithdrawEffects {
        shares_to_burn,
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest,
    } = withdraw_effects;

    // this needs to be i128 as it can be positive if the leftover from the disinvested liquidity is less than withdrawn from available liquidity and negative if the leftover is more than withdrawn from available liquidity
    let token_vault_diff: i128 = i128::from(amounts_before.vault_token_balance)
        - i128::from(amounts_after.vault_token_balance);
    let ctoken_vault_decrease =
        amounts_before.vault_ctoken_balance - amounts_after.vault_ctoken_balance;

    let user_ata_increase = i128::from(amounts_after.user_token_balance)
        - i128::from(amounts_before.user_token_balance);
    let user_shares_diff = amounts_before.user_shares_balance - amounts_after.user_shares_balance;
    let reserve_supply_liquidity_diff = i128::from(amounts_before.reserve_supply_liquidity_balance)
        - i128::from(amounts_after.reserve_supply_liquidity_balance);

    let total_amount_sent_to_user =
        i128::from(available_to_send_to_user) + i128::from(invested_liquidity_to_send_to_user);

    require_msg!(
        total_amount_sent_to_user == reserve_supply_liquidity_diff + token_vault_diff,
        KaminoVaultError::AmountToWithdrawDoesNotMatch,
        &format!(
            "Amount to send to user and result are diff {total_amount_sent_to_user} {}",
            reserve_supply_liquidity_diff + token_vault_diff
        )
    );

    require_msg!(
        ctoken_vault_decrease == invested_to_disinvest_ctokens,
        KaminoVaultError::LiquidityToWithdrawDoesNotMatch,
        &format!("C token amounts to disinvest and result are diff {ctoken_vault_decrease} {invested_to_disinvest_ctokens}")
    );

    require_msg!(
        user_ata_increase == total_amount_sent_to_user,
        KaminoVaultError::UserReceivedAmountDoesNotMatch,
        &format!("User ata diff and expected {user_ata_increase} {total_amount_sent_to_user}",)
    );

    require_msg!(
        user_shares_diff == shares_to_burn,
        KaminoVaultError::SharesBurnedAmountDoesNotMatch,
        &format!("Shares ata diff and result are diff {user_shares_diff} {shares_to_burn}")
    );

    require_msg!(
        reserve_supply_liquidity_diff == i128::from(invested_liquidity_to_disinvest),
        KaminoVaultError::DisinvestedLiquidityAmountDoesNotMatch,
        &format!(
            "Reserve liquidity diff and result are diff {reserve_supply_liquidity_diff} {}",
            invested_liquidity_to_disinvest
        )
    );

    Ok(())
}

pub fn post_transfer_withdraw_pending_fees_balance_checks(
    amounts_before: VaultAndUserBalances,
    amounts_after: VaultAndUserBalances,
    withdraw_fees_effects: WithdrawPendingFeesEffects,
) -> Result<()> {
    let WithdrawPendingFeesEffects {
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest,
    } = withdraw_fees_effects;

    // this can be negative if we disinvested more because of rounding and the fees send to the admin are less than the extra amount disinvested
    let token_vault_diff: i128 = i128::from(amounts_before.vault_token_balance)
        - i128::from(amounts_after.vault_token_balance);
    // ctoken_vault_diff is always positive as either we didn't disinvest at all (so the diff is 0) or we disinvested so now there are less cTokens left in the vault
    let ctoken_vault_decrease =
        amounts_before.vault_ctoken_balance - amounts_after.vault_ctoken_balance;
    let reserve_supply_liquidity_diff = i128::from(amounts_before.reserve_supply_liquidity_balance)
        - i128::from(amounts_after.reserve_supply_liquidity_balance);

    let admin_ata_diff = i128::from(amounts_after.user_token_balance)
        - i128::from(amounts_before.user_token_balance);

    let total_amount_sent_to_user =
        i128::from(available_to_send_to_user) + i128::from(invested_liquidity_to_send_to_user);

    require_msg!(
        total_amount_sent_to_user == reserve_supply_liquidity_diff + token_vault_diff,
        KaminoVaultError::TooMuchLiquidityToWithdraw,
        &format!(
            "Available amounts to withdraw and result are diff {total_amount_sent_to_user} {}",
            reserve_supply_liquidity_diff + token_vault_diff
        )
    );

    require_msg!(
        ctoken_vault_decrease == invested_to_disinvest_ctokens,
        KaminoVaultError::TooMuchLiquidityToWithdraw,
        &format!("C token amounts to disinvest and result are diff {ctoken_vault_decrease} {invested_to_disinvest_ctokens}")
    );

    require_msg!(
        admin_ata_diff == total_amount_sent_to_user,
        KaminoVaultError::TooMuchLiquidityToWithdraw,
        &format!("User ata diff and expected  {admin_ata_diff} {total_amount_sent_to_user}",)
    );

    require_msg!(
        reserve_supply_liquidity_diff == i128::from(invested_liquidity_to_disinvest),
        KaminoVaultError::TooMuchLiquidityToWithdraw,
        &format!(
            "Reserve liquidity diff and result are diff {reserve_supply_liquidity_diff} {}",
            invested_liquidity_to_disinvest
        )
    );

    Ok(())
}

pub fn post_transfer_invest_checks(
    amounts_before: VaultBalances,
    amounts_after: VaultBalances,
    invest_effects: InvestEffects,
    aum_before: Fraction,
    aum_after: Fraction,
) -> Result<()> {
    let InvestEffects {
        direction,
        liquidity_amount,
        collateral_amount,
        rounding_loss,
    } = invest_effects;

    match direction {
        InvestingDirection::Add => {
            require_eq!(
                amounts_before.vault_token_balance - liquidity_amount,
                amounts_after.vault_token_balance - rounding_loss
            );
            require_eq!(
                amounts_before.vault_ctoken_balance + collateral_amount,
                amounts_after.vault_ctoken_balance
            );
            require_eq!(
                amounts_before.reserve_supply_liquidity_balance + liquidity_amount,
                amounts_after.reserve_supply_liquidity_balance
            );
        }
        InvestingDirection::Subtract => {
            require_eq!(
                amounts_before.vault_token_balance + liquidity_amount,
                amounts_after.vault_token_balance - rounding_loss
            );
            require_eq!(
                amounts_before.vault_ctoken_balance - collateral_amount,
                amounts_after.vault_ctoken_balance
            );
            require_eq!(
                amounts_before.reserve_supply_liquidity_balance - liquidity_amount,
                amounts_after.reserve_supply_liquidity_balance
            );
        }
    }

    require!(
        aum_after.ge(&aum_before),
        KaminoVaultError::AUMDecreasedAfterInvest
    );

    Ok(())
}

use kamino_lending::CollateralExchangeRate;

use crate::{
    operations::effects::{DepositIntoReserveEffects, RoundingAware, WithdrawFromReserveEffects},
    utils::fraction_utils::{to_ceil_and_rounding_error, to_floor_and_rounding_error},
};



pub fn compute_reserve_min_deposit_effects(
    exchange_rate: CollateralExchangeRate,
    min_deposit_liquidity: u64,
) -> RoundingAware<DepositIntoReserveEffects> {
    let floored_ctokens = exchange_rate.liquidity_to_collateral(min_deposit_liquidity);

    let (liquidity_for_floored_ctokens, _) = to_ceil_and_rounding_error(
        exchange_rate.fraction_collateral_to_liquidity_ceil(floored_ctokens.into()),
    );

    let ctokens_to_mint = if liquidity_for_floored_ctokens < min_deposit_liquidity {
        floored_ctokens + 1
    } else {
        floored_ctokens
    };

    compute_liquidity_to_deposit_for_ctokens(exchange_rate, ctokens_to_mint)
}





pub fn compute_reserve_max_deposit_effects(
    exchange_rate: CollateralExchangeRate,
    max_deposit_liquidity: u64,
) -> Option<RoundingAware<DepositIntoReserveEffects>> {
    let expected_minted_ctokens = exchange_rate.liquidity_to_collateral(max_deposit_liquidity);

    if expected_minted_ctokens == 0 {
        return None;
    }

    Some(compute_liquidity_to_deposit_for_ctokens(
        exchange_rate,
        expected_minted_ctokens,
    ))
}


pub(super) fn compute_liquidity_to_deposit_for_ctokens(
    exchange_rate: CollateralExchangeRate,
    expected_minted_ctokens: u64,
) -> RoundingAware<DepositIntoReserveEffects> {
    let theoretically_required_liquidity =
        exchange_rate.fraction_collateral_to_liquidity_ceil(expected_minted_ctokens.into());
    let (liquidity_to_deposit, rounding_loss_liquidity) =
        to_ceil_and_rounding_error(theoretically_required_liquidity);
    RoundingAware {
        effect: DepositIntoReserveEffects::new(liquidity_to_deposit, expected_minted_ctokens),
        rounding_loss_liquidity,
    }
}



pub fn compute_reserve_min_withdraw_effects(
    exchange_rate: CollateralExchangeRate,
    min_withdraw_liquidity: u64,
) -> RoundingAware<WithdrawFromReserveEffects> {
    let ctokens_to_redeem = exchange_rate.liquidity_to_collateral_ceil(min_withdraw_liquidity);

    let theoretically_expected_liquidity =
        exchange_rate.fraction_collateral_to_liquidity(ctokens_to_redeem.into());
    let (expected_withdrawn_liquidity, rounding_loss_liquidity) =
        to_floor_and_rounding_error(theoretically_expected_liquidity);

    RoundingAware {
        effect: WithdrawFromReserveEffects::new(ctokens_to_redeem, expected_withdrawn_liquidity),
        rounding_loss_liquidity,
    }
}

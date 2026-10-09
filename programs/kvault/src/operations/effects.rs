use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use kamino_lending::fraction::Fraction;

#[derive(Debug)]
pub struct DepositEffects {
    pub shares_to_mint: u64,
    pub token_to_deposit: u64,
    pub crank_funds_to_deposit: u64,
}

#[derive(Debug, Default)]
pub struct WithdrawEffects {
    pub shares_to_burn: u64,
    pub available_to_send_to_user: u64,
    pub invested_to_disinvest_ctokens: u64,
    pub invested_liquidity_to_send_to_user: u64,
    pub invested_liquidity_to_disinvest: u64,
}

#[derive(Debug, Default)]
pub struct WithdrawPendingFeesEffects {
    pub available_to_send_to_user: u64,
    pub invested_to_disinvest_ctokens: u64,
    pub invested_liquidity_to_send_to_user: u64,
    pub invested_liquidity_to_disinvest: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum InvestingDirection {
    Add,
    Subtract,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvestEffects {
    pub direction: InvestingDirection,
    pub liquidity_amount: u64,
    pub collateral_amount: u64,
    pub rounding_loss: u64,
}


#[derive(Debug, Eq, PartialEq)]
pub struct DepositIntoReserveEffects {

    pub liquidity_to_deposit: u64,


    pub expected_minted_ctokens: u64,
}

impl DepositIntoReserveEffects {

    pub fn new(positive_liquidity_to_deposit: u64, positive_expected_minted_ctokens: u64) -> Self {
        if positive_liquidity_to_deposit > 0 && positive_expected_minted_ctokens > 0 {
            return Self {
                liquidity_to_deposit: positive_liquidity_to_deposit,
                expected_minted_ctokens: positive_expected_minted_ctokens,
            };
        }
        panic!("Invalid deposit into reserve effects: liquidity_to_deposit: {}, expected_minted_ctokens: {}", positive_liquidity_to_deposit, positive_expected_minted_ctokens);
    }
}


#[derive(Debug, Eq, PartialEq)]
pub struct WithdrawFromReserveEffects {

    pub ctokens_to_redeem: u64,


    pub expected_withdrawn_liquidity: u64,
}

impl WithdrawFromReserveEffects {

    pub fn new(
        positive_ctokens_to_redeem: u64,
        positive_expected_withdrawn_liquidity: u64,
    ) -> Self {
        if positive_ctokens_to_redeem > 0 && positive_expected_withdrawn_liquidity > 0 {
            return Self {
                ctokens_to_redeem: positive_ctokens_to_redeem,
                expected_withdrawn_liquidity: positive_expected_withdrawn_liquidity,
            };
        }
        panic!("Invalid withdraw from reserve effects: ctokens_to_redeem: {}, expected_withdrawn_liquidity: {}", positive_ctokens_to_redeem, positive_expected_withdrawn_liquidity);
    }
}


#[derive(Debug, Default, Eq, PartialEq)]
pub struct RoundingAware<T> {

    pub effect: T,


    pub rounding_loss_liquidity: Fraction,
}

#[derive(Debug, PartialEq)]
pub struct RedeemInKindEffects {
    pub shares_to_burn: u64,
    pub ctokens_to_send_to_user: u64,
    pub actual_liquidity_value: Fraction,
    pub vault_aum_before: Fraction,
}

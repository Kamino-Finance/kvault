use anchor_lang::{AnchorDeserialize, AnchorSerialize};

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

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub enum InvestingDirection {
    Add,
    Subtract,
}

#[derive(Debug)]
pub struct InvestEffects {
    pub direction: InvestingDirection,
    pub liquidity_amount: u64,
    pub collateral_amount: u64,
    pub rounding_loss: u64,
}

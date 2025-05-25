use anchor_lang::prelude::*;

// Events
#[event]
pub struct DepositUserAtaBalanceEvent {
    pub user_ata_balance: u64,
}

#[event]
pub struct DepositResultEvent {
    pub shares_to_mint: u64,
    pub token_to_deposit: u64,
    pub crank_funds_to_deposit: u64,
}

#[event]
pub struct SharesToWithdrawEvent {
    pub shares_amount: u64,
    pub user_shares_before: u64,
}

#[event]
pub struct WithdrawResultEvent {
    pub shares_to_burn: u64,
    pub available_to_send_to_user: u64,
    pub invested_to_disinvest_ctokens: u64,
    pub invested_liquidity_to_send_to_user: u64,
}

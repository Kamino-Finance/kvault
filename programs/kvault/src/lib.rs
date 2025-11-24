#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

pub mod events;
mod handlers;
pub mod operations;
pub mod state;
pub mod utils;

use crate::handlers::*;
pub use crate::operations::reserve_whitelist_operations::UpdateReserveWhitelistMode;
pub use crate::operations::vault_config_operations::VaultConfigField;
pub use crate::state::*;

#[cfg(feature = "staging")]
declare_id!("stKvQfwRsQiKnLtMNVLHKS3exFJmZFsgfzBPWHECUYK");

#[cfg(not(feature = "staging"))]
declare_id!("KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Kamino Lending Vault",
    project_url: "https://kamino.finance/",
    contacts: "email:security@kamino.finance",
    policy: "https://github.com/Kamino-Finance/audits/blob/master/docs/SECURITY.md",

    // Optional Fields
    preferred_languages: "en",
    auditors: "OtterSec, Offside Labs, Certora, Sec3"
}

#[program]
pub mod kamino_vault {

    use super::*;

    pub fn init_vault(ctx: Context<InitVault>) -> Result<()> {
        handler_init_vault::process(ctx)
    }

    pub fn update_reserve_allocation(
        ctx: Context<UpdateReserveAllocation>,
        weight: u64,
        cap: u64,
    ) -> Result<()> {
        handler_update_reserve_allocation::process(ctx, weight, cap)
    }

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, Deposit<'info>>,
        max_amount: u64,
    ) -> Result<()> {
        handler_deposit::process(ctx, max_amount)
    }

    pub fn buy<'info>(
        ctx: Context<'_, '_, '_, 'info, Deposit<'info>>,
        max_amount: u64,
    ) -> Result<()> {
        // Interface to buy vault tokens, to be improved
        // later to go through DEXes also
        handler_deposit::process(ctx, max_amount)
    }

    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>,
        shares_amount: u64,
    ) -> Result<()> {
        handler_withdraw::withdraw(ctx, shares_amount)
    }

    pub fn sell<'info>(
        ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>,
        shares_amount: u64,
    ) -> Result<()> {
        // Interface to sell vault tokens, to be improved
        // later to go through DEXes also
        handler_withdraw::withdraw(ctx, shares_amount)
    }

    pub fn invest<'info>(ctx: Context<'_, '_, '_, 'info, Invest<'info>>) -> Result<()> {
        handler_invest::process(ctx)
    }

    pub fn update_vault_config<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateVaultConfig<'info>>,
        entry: VaultConfigField,
        data: Vec<u8>,
    ) -> Result<()> {
        handler_update_vault_config::process(ctx, entry, &data)
    }

    pub fn withdraw_pending_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawPendingFees<'info>>,
    ) -> Result<()> {
        handler_withdraw_pending_fees::process(ctx)
    }

    pub fn update_admin<'info>(ctx: Context<'_, '_, '_, 'info, UpdateAdmin<'info>>) -> Result<()> {
        handler_update_admin::process(ctx)
    }

    pub fn give_up_pending_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, GiveUpPendingFees<'info>>,
        max_amount_to_give_up: u64,
    ) -> Result<()> {
        handler_give_up_pending_fees::process(ctx, max_amount_to_give_up)
    }

    pub fn initialize_shares_metadata<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeSharesMetadata<'info>>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        handler_initialize_shares_metadata::process(ctx, name, symbol, uri)
    }

    pub fn update_shares_metadata<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateSharesMetadata<'info>>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        handler_update_metadata::process(ctx, name, symbol, uri)
    }

    pub fn withdraw_from_available<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawFromAvailable<'info>>,
        shares_amount: u64,
    ) -> Result<()> {
        handler_withdraw::withdraw_from_available(ctx, shares_amount)
    }

    pub fn remove_allocation(ctx: Context<RemoveAllocation>) -> Result<()> {
        handler_remove_allocation::process(ctx)
    }

    pub fn init_global_config(ctx: Context<InitGlobalConfig>) -> Result<()> {
        handler_initialize_global_config::process(ctx)
    }

    pub fn update_global_config(
        ctx: Context<UpdateGlobalConfig>,
        update: utils::global_config::UpdateGlobalConfigMode,
    ) -> Result<()> {
        handler_update_global_config::process(ctx, update)
    }

    pub fn update_global_config_admin(ctx: Context<UpdateGlobalConfigAdmin>) -> Result<()> {
        handler_update_global_config_admin::process(ctx)
    }

    pub fn add_update_whitelisted_reserve(
        ctx: Context<AddUpdateWhitelistedReserve>,
        update: UpdateReserveWhitelistMode,
    ) -> Result<()> {
        handler_add_update_whitelisted_reserve::process(ctx, update)
    }
}

#[error_code]
#[derive(PartialEq, Eq, strum::EnumString)]
// ..
pub enum KaminoVaultError {
    #[msg("Cannot deposit zero tokens")]
    DepositAmountsZero = 1000,

    #[msg("Post check failed on share issued")]
    SharesIssuedAmountDoesNotMatch,

    #[msg("Math operation overflowed")]
    MathOverflow,

    #[msg("Integer conversion overflowed")]
    IntegerOverflow,

    #[msg("Withdrawn amount is below minimum")]
    WithdrawAmountBelowMinimum,

    #[msg("TooMuchLiquidityToWithdraw")]
    TooMuchLiquidityToWithdraw,

    #[msg("ReserveAlreadyExists")]
    ReserveAlreadyExists,

    #[msg("ReserveNotPartOfAllocations")]
    ReserveNotPartOfAllocations,

    #[msg("CouldNotDeserializeAccountAsReserve")]
    CouldNotDeserializeAccountAsReserve,

    #[msg("ReserveNotProvidedInTheAccounts")]
    ReserveNotProvidedInTheAccounts,

    #[msg("ReserveAccountAndKeyMismatch")]
    ReserveAccountAndKeyMismatch,

    #[msg("OutOfRangeOfReserveIndex")]
    OutOfRangeOfReserveIndex,

    #[msg("OutOfRangeOfReserveIndex")]
    CannotFindReserveInAllocations,

    #[msg("Invested amount is below minimum")]
    InvestAmountBelowMinimum,

    #[msg("AdminAuthorityIncorrect")]
    AdminAuthorityIncorrect,

    #[msg("BaseVaultAuthorityIncorrect")]
    BaseVaultAuthorityIncorrect,

    #[msg("BaseVaultAuthorityBumpIncorrect")]
    BaseVaultAuthorityBumpIncorrect,

    #[msg("TokenMintIncorrect")]
    TokenMintIncorrect,

    #[msg("TokenMintDecimalsIncorrect")]
    TokenMintDecimalsIncorrect,

    #[msg("TokenVaultIncorrect")]
    TokenVaultIncorrect,

    #[msg("SharesMintDecimalsIncorrect")]
    SharesMintDecimalsIncorrect,

    #[msg("SharesMintIncorrect")]
    SharesMintIncorrect,

    #[msg("InitialAccountingIncorrect")]
    InitialAccountingIncorrect,

    #[msg("Reserve is stale and must be refreshed before any operation")]
    ReserveIsStale,

    #[msg("Not enough liquidity disinvested to send to user")]
    NotEnoughLiquidityDisinvestedToSendToUser,

    #[msg("BPS value is greater than 10000")]
    BPSValueTooBig,

    #[msg("Deposited amount is below minimum")]
    DepositAmountBelowMinimum,

    #[msg("Vault have no space for new reserves")]
    ReserveSpaceExhausted,

    #[msg("Cannot withdraw from empty vault")]
    CannotWithdrawFromEmptyVault,

    #[msg("TokensDepositedAmountDoesNotMatch")]
    TokensDepositedAmountDoesNotMatch,

    #[msg("Amount to withdraw does not match")]
    AmountToWithdrawDoesNotMatch,

    #[msg("Liquidity to withdraw does not match")]
    LiquidityToWithdrawDoesNotMatch,

    #[msg("User received amount does not match")]
    UserReceivedAmountDoesNotMatch,

    #[msg("Shares burned amount does not match")]
    SharesBurnedAmountDoesNotMatch,

    #[msg("Disinvested liquidity amount does not match")]
    DisinvestedLiquidityAmountDoesNotMatch,

    #[msg("SharesMintedAmountDoesNotMatch")]
    SharesMintedAmountDoesNotMatch,

    #[msg("AUM decreased after invest")]
    AUMDecreasedAfterInvest,

    #[msg("AUM is below pending fees")]
    AUMBelowPendingFees,

    #[msg("Deposit amount results in 0 shares")]
    DepositAmountsZeroShares,

    #[msg("Withdraw amount results in 0 shares")]
    WithdrawResultsInZeroShares,

    #[msg("Cannot withdraw zero shares")]
    CannotWithdrawZeroShares,

    #[msg("Management fee is greater than maximum allowed")]
    ManagementFeeGreaterThanMaxAllowed,

    #[msg("Vault assets under management are empty")]
    VaultAUMZero,

    #[msg("Missing reserve for batch refresh")]
    MissingReserveForBatchRefresh,

    #[msg("Min withdraw amount is too big")]
    MinWithdrawAmountTooBig,

    #[msg("Invest is called too soon after last invest")]
    InvestTooSoon,

    #[msg("Wrong admin or allocation admin")]
    WrongAdminOrAllocationAdmin,

    #[msg("Reserve has non-zero allocation or ctokens so cannot be removed")]
    ReserveHasNonZeroAllocationOrCTokens,

    #[msg("Deposit amount is greater than requested amount")]
    DepositAmountGreaterThanRequestedAmount,

    #[msg("Withdraw amount is less than withdrawal penalty")]
    WithdrawAmountLessThanWithdrawalPenalty,

    #[msg("Cannot withdraw 0 lamports")]
    CannotWithdrawZeroLamports,

    #[msg("Cannot initialize global config because there is no upgrade authority to the program")]
    NoUpgradeAuthority,

    #[msg("Withdrawal fee BPS is greater than maximum allowed")]
    WithdrawalFeeBPSGreaterThanMaxAllowed,

    #[msg("Withdrawal fee lamports is greater than maximum allowed")]
    WithdrawalFeeLamportsGreaterThanMaxAllowed,

    #[msg("Reserve is not whitelisted")]
    ReserveNotWhitelisted,

    #[msg("Invalid bool-like value passed in (should be 0 or 1)")]
    InvalidBoolLikeValue,
}

pub type KaminoVaultResult<T = ()> = std::result::Result<T, KaminoVaultError>;


//! Kvault program error codes.
//!
//! Anchor custom errors start at offset 6000. Kvault errors use offsets starting at 1000,
//! so error codes start at 7000. Each variant's error code is `6000 + offset`.
//! Use [`KvaultError::from_error_code`] to convert an on-chain error code back to a variant,
//! or `TryFrom<u32>` for the same purpose.
//!
//! # Example
//!
//! ```rust
//! use kvault_interface::KvaultError;
//!
//! let error = KvaultError::from_error_code(7002);
//! assert_eq!(error, Some(KvaultError::MathOverflow));
//! assert_eq!(error.unwrap().error_code(), 7002);
//! assert_eq!(
//!     error.unwrap().to_string(),
//!     "Math operation overflowed"
//! );
//! ```

/// Anchor error-code base for custom program errors.
const ANCHOR_ERROR_BASE: u32 = 6000;

macro_rules! define_kvault_errors {
    (
        $(
            $(#[doc = $doc:expr])*
            $variant:ident = $offset:literal => $msg:expr
        ),*
        $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        pub enum KvaultError {
            $(
                $(#[doc = $doc])*
                $variant = ANCHOR_ERROR_BASE + $offset,
            )*
        }

        impl KvaultError {
            /// Returns the Anchor error code for this variant (`6000 + offset`).
            pub const fn error_code(self) -> u32 {
                self as u32
            }

            /// Returns the human-readable error message.
            pub const fn message(self) -> &'static str {
                match self {
                    $(Self::$variant => $msg,)*
                }
            }

            /// Converts an Anchor error code to a `KvaultError`, if it matches a known variant.
            pub const fn from_error_code(code: u32) -> Option<Self> {
                match code {
                    $(x if x == ANCHOR_ERROR_BASE + $offset => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }

        impl core::fmt::Display for KvaultError {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.message())
            }
        }

        impl TryFrom<u32> for KvaultError {
            type Error = u32;

            /// Converts an error code to a `KvaultError`.
            /// Returns `Err(code)` if the code doesn't match any known variant.
            fn try_from(code: u32) -> Result<Self, Self::Error> {
                Self::from_error_code(code).ok_or(code)
            }
        }
    };
}

define_kvault_errors! {
    /// Deposit amounts are zero.
    DepositAmountsZero = 1000 => "Cannot deposit zero tokens",
    /// Shares issued amount does not match expected.
    SharesIssuedAmountDoesNotMatch = 1001 => "Post check failed on share issued",
    /// Math operation overflowed.
    MathOverflow = 1002 => "Math operation overflowed",
    /// Integer conversion overflowed.
    IntegerOverflow = 1003 => "Integer conversion overflowed",
    /// Withdraw amount is below the minimum.
    WithdrawAmountBelowMinimum = 1004 => "Withdrawn amount is below minimum",
    /// Too much liquidity requested for withdrawal.
    TooMuchLiquidityToWithdraw = 1005 => "TooMuchLiquidityToWithdraw",
    /// Reserve already exists in the vault.
    ReserveAlreadyExists = 1006 => "ReserveAlreadyExists",
    /// Reserve is not part of vault allocations.
    ReserveNotPartOfAllocations = 1007 => "ReserveNotPartOfAllocations",
    /// Could not deserialize account as a reserve.
    CouldNotDeserializeAccountAsReserve = 1008 => "CouldNotDeserializeAccountAsReserve",
    /// Reserve not provided in the accounts.
    ReserveNotProvidedInTheAccounts = 1009 => "ReserveNotProvidedInTheAccounts",
    /// Reserve account and key mismatch.
    ReserveAccountAndKeyMismatch = 1010 => "ReserveAccountAndKeyMismatch",
    /// Reserve index is out of range.
    OutOfRangeOfReserveIndex = 1011 => "OutOfRangeOfReserveIndex",
    /// Cannot find reserve in allocations.
    CannotFindReserveInAllocations = 1012 => "OutOfRangeOfReserveIndex",
    /// Invest amount is below the minimum.
    InvestAmountBelowMinimum = 1013 => "Invested amount is below minimum",
    /// Admin authority is incorrect.
    AdminAuthorityIncorrect = 1014 => "AdminAuthorityIncorrect",
    /// Base vault authority is incorrect.
    BaseVaultAuthorityIncorrect = 1015 => "BaseVaultAuthorityIncorrect",
    /// Base vault authority bump is incorrect.
    BaseVaultAuthorityBumpIncorrect = 1016 => "BaseVaultAuthorityBumpIncorrect",
    /// Token mint is incorrect.
    TokenMintIncorrect = 1017 => "TokenMintIncorrect",
    /// Token mint decimals are incorrect.
    TokenMintDecimalsIncorrect = 1018 => "TokenMintDecimalsIncorrect",
    /// Token vault is incorrect.
    TokenVaultIncorrect = 1019 => "TokenVaultIncorrect",
    /// Shares mint decimals are incorrect.
    SharesMintDecimalsIncorrect = 1020 => "SharesMintDecimalsIncorrect",
    /// Shares mint is incorrect.
    SharesMintIncorrect = 1021 => "SharesMintIncorrect",
    /// Initial accounting is incorrect.
    InitialAccountingIncorrect = 1022 => "InitialAccountingIncorrect",
    /// Reserve state needs to be refreshed.
    ReserveIsStale = 1023 => "Reserve is stale and must be refreshed before any operation",
    /// Not enough liquidity disinvested to send to user.
    NotEnoughLiquidityDisinvestedToSendToUser = 1024 => "Not enough liquidity disinvested to send to user",
    /// Basis points value is too large.
    BPSValueTooBig = 1025 => "BPS value is greater than 10000",
    /// Deposit amount is below the minimum.
    DepositAmountBelowMinimum = 1026 => "Deposited amount is below minimum",
    /// No more reserve slots available in the vault.
    ReserveSpaceExhausted = 1027 => "Vault have no space for new reserves",
    /// Cannot withdraw from an empty vault.
    CannotWithdrawFromEmptyVault = 1028 => "Cannot withdraw from empty vault",
    /// Tokens deposited amount does not match expected.
    TokensDepositedAmountDoesNotMatch = 1029 => "TokensDepositedAmountDoesNotMatch",
    /// Amount to withdraw does not match expected.
    AmountToWithdrawDoesNotMatch = 1030 => "Amount to withdraw does not match",
    /// Liquidity to withdraw does not match expected.
    LiquidityToWithdrawDoesNotMatch = 1031 => "Liquidity to withdraw does not match",
    /// User received amount does not match expected.
    UserReceivedAmountDoesNotMatch = 1032 => "User received amount does not match",
    /// Shares burned amount does not match expected.
    SharesBurnedAmountDoesNotMatch = 1033 => "Shares burned amount does not match",
    /// Disinvested liquidity amount does not match expected.
    DisinvestedLiquidityAmountDoesNotMatch = 1034 => "Disinvested liquidity amount does not match",
    /// Shares minted amount does not match expected.
    SharesMintedAmountDoesNotMatch = 1035 => "SharesMintedAmountDoesNotMatch",
    /// Assets under management decreased after invest.
    AUMDecreasedAfterInvest = 1036 => "AUM decreased after invest",
    /// Assets under management is below pending fees.
    AUMBelowPendingFees = 1037 => "AUM is below pending fees",
    /// Deposit would result in zero shares.
    DepositAmountsZeroShares = 1038 => "Deposit amount results in 0 shares",
    /// Withdraw results in zero shares.
    WithdrawResultsInZeroShares = 1039 => "Withdraw amount results in 0 shares",
    /// Cannot withdraw zero shares.
    CannotWithdrawZeroShares = 1040 => "Cannot withdraw zero shares",
    /// Management fee exceeds the maximum allowed.
    ManagementFeeGreaterThanMaxAllowed = 1041 => "Management fee is greater than maximum allowed",
    /// Vault assets under management is zero.
    VaultAUMZero = 1042 => "Vault assets under management are empty",
    /// Missing reserve account for batch refresh.
    MissingReserveForBatchRefresh = 1043 => "Missing reserve for batch refresh",
    /// Minimum withdraw amount is too large.
    MinWithdrawAmountTooBig = 1044 => "Min withdraw amount is too big",
    /// Invest called too soon after the previous invest.
    InvestTooSoon = 1045 => "Invest is called too soon after last invest",
    /// Signer is neither the admin nor the allocation admin.
    WrongAdminOrAllocationAdmin = 1046 => "Wrong admin or allocation admin",
    /// Reserve has non-zero allocation or cToken balance.
    ReserveHasNonZeroAllocationOrCTokens = 1047 => "Reserve has non-zero allocation or ctokens so cannot be removed",
    /// Deposit amount exceeds the requested amount.
    DepositAmountGreaterThanRequestedAmount = 1048 => "Deposit amount is greater than requested amount",
    /// Withdraw amount is less than the withdrawal penalty.
    WithdrawAmountLessThanWithdrawalPenalty = 1049 => "Withdraw amount is less than withdrawal penalty",
    /// Cannot withdraw zero lamports.
    CannotWithdrawZeroLamports = 1050 => "Cannot withdraw 0 lamports",
    /// Program has no upgrade authority.
    NoUpgradeAuthority = 1051 => "Cannot initialize global config because there is no upgrade authority to the program",
    /// Withdrawal fee BPS exceeds the maximum allowed.
    WithdrawalFeeBPSGreaterThanMaxAllowed = 1052 => "Withdrawal fee BPS is greater than maximum allowed",
    /// Withdrawal fee lamports exceeds the maximum allowed.
    WithdrawalFeeLamportsGreaterThanMaxAllowed = 1053 => "Withdrawal fee lamports is greater than maximum allowed",
    /// Reserve is not whitelisted.
    ReserveNotWhitelisted = 1054 => "Reserve is not whitelisted",
    /// Invalid boolean-like value.
    InvalidBoolLikeValue = 1055 => "Invalid bool-like value passed in (should be 0 or 1)",
    /// Assets under management decreased more than expected.
    AUMDecreasedMoreThanExpected = 1056 => "AUM decreased more than expected during redeem in kind",
    RewardTopupAmountZero = 1057 => "Reward topup amount is zero",
    RewardTopupAmountNotExpected = 1058 => "Reward topup is not as expected after transfer",
    RewardWithdrawAmountZero = 1059 => "Reward withdraw amount is zero",
    RewardWithdrawAmountNotExpected = 1060 => "Reward withdraw is not as expected after transfer",
    RewardsStaleForFeeUpdate = 1061 => "Rewards are stale - must be refreshed before updating fees",
    /// Vault deposit cap reached.
    VaultDepositCapReached = 1062 => "Vault deposit cap reached",
    /// Max invest amount must be greater than zero.
    MaxInvestAmountMustBeGreaterThanZero = 1063 => "max_amount must be greater than 0",
        /// Shares minted are below the requested minimum.
    SharesOutBelowMinimum = 1064 => "Shares out is below minimum requested",
}

impl std::error::Error for KvaultError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_error_codes() {
        assert_eq!(KvaultError::DepositAmountsZero.error_code(), 7000);
        assert_eq!(KvaultError::MathOverflow.error_code(), 7002);
        assert_eq!(KvaultError::AUMDecreasedMoreThanExpected.error_code(), 7056);
        assert_eq!(KvaultError::RewardsStaleForFeeUpdate.error_code(), 7061);
        assert_eq!(KvaultError::VaultDepositCapReached.error_code(), 7062);
        assert_eq!(
            KvaultError::MaxInvestAmountMustBeGreaterThanZero.error_code(),
            7063
        );
        assert_eq!(KvaultError::SharesOutBelowMinimum.error_code(), 7064);

        assert_eq!(
            KvaultError::from_error_code(7000),
            Some(KvaultError::DepositAmountsZero)
        );
        assert_eq!(
            KvaultError::from_error_code(7056),
            Some(KvaultError::AUMDecreasedMoreThanExpected)
        );
        assert_eq!(KvaultError::from_error_code(5000), None);
        assert_eq!(KvaultError::from_error_code(9999), None);

        // Display shows human-readable message
        assert_eq!(
            KvaultError::MathOverflow.to_string(),
            "Math operation overflowed"
        );

        // TryFrom works the same as from_error_code
        assert_eq!(KvaultError::try_from(7002), Ok(KvaultError::MathOverflow));
        assert_eq!(KvaultError::try_from(9999), Err(9999));
    }
}

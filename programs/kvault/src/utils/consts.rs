pub const TOKEN_VAULT_SEED: &[u8; 11] = b"token_vault";
pub const CTOKEN_VAULT_SEED: &[u8; 12] = b"ctoken_vault";
pub const BASE_VAULT_AUTHORITY_SEED: &[u8; 9] = b"authority";
pub const SHARES_SEEDS: &[u8; 6] = b"shares";
pub const METADATA_SEEDS: &[u8; 8] = b"metadata";
pub const EVENT_AUTHORITY: &[u8] = b"__event_authority";
pub const GLOBAL_CONFIG_STATE_SEEDS: &[u8] = b"global_config";
pub const WHITELISTED_RESERVES_SEED: &[u8] = b"whitelisted_reserves";

pub const VAULT_STATE_SIZE: usize = 62544;
pub const VAULT_ALLOCATION_SIZE: usize = 2160;
pub const GLOBAL_CONFIG_SIZE: usize = 1024;
pub const RESERVE_WHITELIST_ENTRY_SIZE: usize = 128;

/// Max value for management fee that a vault manager can set
pub const MAX_MGMT_FEE_BPS: u64 = 1000;

pub const SECONDS_PER_YEAR: f64 = 365.242_199 * 24.0 * 60.0 * 60.0;
pub const SECONDS_PER_YEAR_U64: u64 = 31556925;
pub const SECONDS_PER_MINUTE: u64 = 60;
pub const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * 60;
pub const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;

pub const UPPER_LIMIT_MIN_WITHDRAW_AMOUNT: u64 = 1000; // for the most expensive lamport (cbBTC) this can go up to 1$

pub const INITIAL_DEPOSIT_AMOUNT: u64 = 1000;

pub const MAX_WITHDRAWAL_PENALTY_BPS: u64 = 1000;
pub const MAX_WITHDRAWAL_PENALTY_LAMPORTS: u64 = 10_000;

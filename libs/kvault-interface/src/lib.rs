//! # kvault-interface
//!
//! Instruction builders and zero-copy account deserialization for
//! Kamino Vault (Kvault). No `anchor-lang` dependency;
//! targets `solana-sdk` v2.x.
//!
//! ## Quick start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! kvault-interface = { git = "https://github.com/Kamino-Finance/kvault" }
//! klend-interface = { git = "https://github.com/Kamino-Finance/klend" }
//! solana-pubkey = "2.1"
//! solana-instruction = "2.1"
//! solana-sdk = "~2.3"
//! solana-client = "2.1"
//! spl-associated-token-account = "6"
//! ```
//!
//! ## Two-level API
//!
//! The crate provides two levels of instruction building:
//!
//! - **Low-level** ([`instructions`]): one function per Kvault instruction, returning a single
//!   [`solana_instruction::Instruction`]. You supply every account address manually.
//!
//! - **High-level** ([`helpers`]): workflow builders that accept [`VaultInfo`] /
//!   [`ReserveInfo`] and auto-derive PDAs and remaining accounts.
//!
//! ## Core types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`VaultInfo`] | On-chain vault metadata; built via [`VaultInfo::from_account_data`]. |
//! | [`ReserveInfo`] | Klend reserve metadata needed by withdraw / invest / redeem helpers. |
//! | [`state::VaultState`] | Zero-copy deserialization of the on-chain VaultState account (62 544 bytes). |
//! | [`state::GlobalConfig`] | Zero-copy deserialization of the on-chain GlobalConfig account (1 024 bytes). |
//! | [`state::VaultAllocation`] | Per-reserve allocation slot embedded in VaultState (2 160 bytes). |
//!
//! ## Typical flow
//!
//! ```rust,no_run
//! use kvault_interface::{helpers, VaultInfo, KVAULT_PROGRAM_ID};
//! use solana_pubkey::Pubkey;
//! # fn example(rpc: &solana_client::rpc_client::RpcClient, owner: Pubkey) -> Result<(), Box<dyn std::error::Error>> {
//!
//! let vault_pubkey: Pubkey = "HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E".parse()?;
//!
//! // 1. Fetch the vault account, then the active reserves it allocates to
//! //    (their lending markets are needed to refresh the reserves on-chain).
//! let vault_data = rpc.get_account(&vault_pubkey)?;
//! let vault_state = kvault_interface::from_account_data::<kvault_interface::state::VaultState>(&vault_data.data)?;
//! let mut reserve_infos = Vec::new();
//! for reserve_pubkey in VaultInfo::active_reserve_addresses(vault_state) {
//!     let reserve_data = rpc.get_account(&reserve_pubkey)?;
//!     reserve_infos.push(kvault_interface::ReserveInfo::from_account_data(reserve_pubkey, &reserve_data.data)?);
//! }
//! let vault = VaultInfo::from_account_data(vault_pubkey, &vault_data.data, &reserve_infos)?;
//!
//! // 2. Derive user token accounts
//! let user_token_ata = spl_associated_token_account::get_associated_token_address(&owner, &vault.token_mint);
//! let shares_mint = kvault_interface::pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_pubkey).0;
//! let user_shares_ata = spl_associated_token_account::get_associated_token_address(&owner, &shares_mint);
//!
//! // 3. Build a deposit instruction — PDAs and remaining accounts are derived automatically
//! let ix = helpers::deposit::deposit(&vault, owner, user_token_ata, user_shares_ata, 1_000_000);
//!
//! // 4. Send the transaction
//! let message = solana_sdk::message::Message::new(&[ix], Some(&owner));
//! let recent_blockhash = rpc.get_latest_blockhash()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Scaled fraction fields (`_sf`)
//!
//! All on-chain fields ending in `_sf` (e.g. `prev_aum_sf`, `pending_fees_sf`) are
//! fixed-point integers using the [`Fraction`] type (68 integer bits, 60 fractional bits).
//! Convert with `Fraction::from_bits(u128_value)`.
//!
//! ## Examples
//!
//! The `examples/` directory contains runnable examples matching the
//! [Kamino developer documentation](https://docs.kamino.finance):
//!
//! | Example | Description |
//! |---------|-------------|
//! | [`deposit`](https://github.com/Kamino-Finance/kvault/blob/master/libs/kvault-interface/examples/deposit.rs) | Deposit tokens into a vault and receive shares. |
//! | [`withdraw`](https://github.com/Kamino-Finance/kvault/blob/master/libs/kvault-interface/examples/withdraw.rs) | Withdraw shares from a vault to receive tokens. |
//! | [`vault_data`](https://github.com/Kamino-Finance/kvault/blob/master/libs/kvault-interface/examples/vault_data.rs) | Fetch and inspect vault state, allocations, and AUM. |
//! | [`user_position`](https://github.com/Kamino-Finance/kvault/blob/master/libs/kvault-interface/examples/user_position.rs) | Read a user's share balance and compute token value. |

pub mod discriminators;
pub mod errors;
pub mod helpers;
pub mod instructions;
pub mod pda;
pub mod state;
pub mod util;

// Convenience re-exports for the most commonly used types.
pub use errors::KvaultError;
pub use helpers::info::{ReserveInfo, VaultInfo, VaultInfoError, VaultReserve};
pub use state::{from_account_data, AccountDataError};

use solana_pubkey::{pubkey, Pubkey};

/// Fixed-point type for `_sf` (scaled fraction) fields: 68 integer bits, 60 fractional bits.
///
/// All on-chain values ending in `_sf` (e.g. `prev_aum_sf`, `pending_fees_sf`) are
/// stored as `u128` / `PodU128` bit patterns of this type. Convert with
/// `Fraction::from_bits(u128_value)` and `fraction.to_bits()`.
pub type Fraction = fixed::types::U68F60;

/// Maximum number of reserves a vault can allocate to.
pub const MAX_RESERVES: usize = 25;

/// Kamino Vault (mainnet) program ID.
pub const KVAULT_PROGRAM_ID: Pubkey = pubkey!("KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd");

/// Kamino Vault (staging) program ID.
pub const KVAULT_STAGING_PROGRAM_ID: Pubkey =
    pubkey!("stKvQfwRsQiKnLtMNVLHKS3exFJmZFsgfzBPWHECUYK");

// Re-export well-known IDs from klend-interface to avoid duplication.
pub use klend_interface::{
    KLEND_PROGRAM_ID, SYSTEM_PROGRAM_ID, SYSVAR_INSTRUCTIONS_ID, SYSVAR_RENT_ID, TOKEN_PROGRAM_ID,
};

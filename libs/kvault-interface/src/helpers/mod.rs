//! High-level instruction builders that auto-derive PDAs and remaining accounts.
//!
//! Each helper accepts a [`VaultInfo`] (and optionally a [`ReserveInfo`]) and
//! returns a single [`solana_instruction::Instruction`] ready to include in a
//! transaction.
//!
//! For the low-level API where you supply every account manually, see
//! [`instructions`](crate::instructions).

mod common;
pub mod deposit;
pub mod info;
pub mod invest;
pub mod redeem;
pub mod withdraw;

pub use deposit::{buy, deposit};
pub use info::*;
pub use invest::invest;
pub use redeem::redeem_in_kind;
pub use withdraw::{sell, withdraw, withdraw_from_available};

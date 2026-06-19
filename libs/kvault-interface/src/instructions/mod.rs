//! Low-level instruction builders — one function per Kvault instruction.
//!
//! Each function takes an `*Accounts` struct and returns a single
//! [`solana_instruction::Instruction`]. The caller supplies every account
//! address manually.
//!
//! For a higher-level API that auto-derives PDAs and remaining accounts,
//! see [`helpers`](crate::helpers).

pub mod deposit;
pub mod invest;
pub mod redeem;
pub mod withdraw;

pub use deposit::*;
pub use invest::*;
pub use redeem::*;
pub use withdraw::*;

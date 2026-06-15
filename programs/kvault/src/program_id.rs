use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::pubkey;

#[cfg(all(feature = "mainnet", feature = "staging"))]
compile_error!("'mainnet' and 'staging' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "mainnet", feature = "staging-to-staging"))]
compile_error!("'mainnet' and 'staging-to-staging' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "staging", feature = "staging-to-staging"))]
compile_error!("'staging' and 'staging-to-staging' features are mutually exclusive.");

#[cfg(all(feature = "mainnet", feature = "devnet"))]
compile_error!("'mainnet' and 'devnet' features are mutually exclusive. Have you missed to disable default features?");

pub const KVAULT_MAINNET_PROGRAM_ID: Pubkey =
    pubkey!("KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd");

pub const KVAULT_STAGING_PROGRAM_ID: Pubkey =
    pubkey!("stKvQfwRsQiKnLtMNVLHKS3exFJmZFsgfzBPWHECUYK");

pub const KVAULT_STAGING_TO_STAGING_PROGRAM_ID: Pubkey =
    pubkey!("st2Kvh82VyY8JskVJi4PebU9vdnR14VsaEy6TWVzD1r");

pub const KVAULT_DEVNET_PROGRAM_ID: Pubkey = pubkey!("devkRngFnfp4gBc5a3LsadgbQKdPo8MSZ4prFiNSVmY");

#[cfg(feature = "mainnet")]
pub const KVAULT_PROGRAM_ID: Pubkey = KVAULT_MAINNET_PROGRAM_ID;

#[cfg(feature = "staging")]
pub const KVAULT_PROGRAM_ID: Pubkey = KVAULT_STAGING_PROGRAM_ID;

#[cfg(feature = "staging-to-staging")]
pub const KVAULT_PROGRAM_ID: Pubkey = KVAULT_STAGING_TO_STAGING_PROGRAM_ID;

#[cfg(feature = "devnet")]
pub const KVAULT_PROGRAM_ID: Pubkey = KVAULT_DEVNET_PROGRAM_ID;

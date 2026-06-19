use bytemuck::{Pod, Zeroable};
use solana_pubkey::Pubkey;
use spl_discriminator::SplDiscriminate;

/// System-wide vault configuration.
#[derive(Debug, Clone, Copy, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("account:GlobalConfig")]
#[repr(C)]
pub struct GlobalConfig {
    /// System-wide admin wallet.
    pub global_admin: Pubkey,
    /// Wallet nominated as the next global admin (two-step transfer).
    pub pending_admin: Pubkey,

    /// Default flat withdrawal penalty in lamports, applied to all vaults.
    pub withdrawal_penalty_lamports: u64,
    /// Default withdrawal penalty in basis points, applied to all vaults.
    pub withdrawal_penalty_bps: u64,

    /// Reserved for future use.
    pub padding: [u8; 944],
}

const _: () = assert!(core::mem::size_of::<GlobalConfig>() == 1024);

use bytemuck::{Pod, Zeroable};
use solana_pubkey::Pubkey;

/// PDA account controlling which Kamino Lending reserves a vault can use.
///
/// On-chain this uses `#[account]` (Borsh), but all fields are fixed-size
/// so the byte layout is identical to C repr after the 8-byte discriminator.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ReserveWhitelistEntry {
    /// SPL token mint of the whitelisted reserve.
    pub token_mint: Pubkey,
    /// Klend reserve account address.
    pub reserve: Pubkey,
    /// When non-zero (`1`), this reserve can be added as an allocation target.
    pub whitelist_add_allocation: u8,
    /// When non-zero (`1`), invest operations are permitted for this reserve.
    pub whitelist_invest: u8,
    /// Reserved for future use.
    pub padding: [u8; 62],
}

const _: () = assert!(core::mem::size_of::<ReserveWhitelistEntry>() == 128);

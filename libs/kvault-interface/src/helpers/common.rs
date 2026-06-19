//! Shared utilities used by the helper instruction builders.

use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use super::info::VaultInfo;
use crate::{
    pda,
    util::{readonly, writable},
    KVAULT_PROGRAM_ID,
};

/// Remaining accounts for refresh, in two slot-ordered blocks:
/// writable reserve metas followed by readonly lending-market metas.
///
/// This mirrors the canonical klend SDK layout consumed by the on-chain
/// `cpi_refresh_reserves`, which builds `[reserve(writable),
/// lending_market(readonly)]` pairs and therefore needs both blocks present.
pub(crate) fn refresh_remaining_accounts(vault: &VaultInfo) -> Vec<AccountMeta> {
    let mut metas = Vec::with_capacity(vault.reserves().len() * 2);
    metas.extend(vault.reserves().iter().map(|r| writable(r.reserve)));
    metas.extend(vault.reserves().iter().map(|r| readonly(r.lending_market)));
    metas
}

/// Pre-derived vault PDAs.
pub(crate) struct VaultPdas {
    pub base_vault_authority: Pubkey,
    pub token_vault: Pubkey,
    pub shares_mint: Pubkey,
}

/// Derive the common vault PDAs from the vault state address.
pub(crate) fn derive_vault_pdas(vault_state: &Pubkey) -> VaultPdas {
    VaultPdas {
        base_vault_authority: pda::base_vault_authority(&KVAULT_PROGRAM_ID, vault_state).0,
        token_vault: pda::token_vault(&KVAULT_PROGRAM_ID, vault_state).0,
        shares_mint: pda::shares_mint(&KVAULT_PROGRAM_ID, vault_state).0,
    }
}

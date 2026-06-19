//! PDA derivation helpers for Kvault accounts.
//!
//! Each function calls [`Pubkey::find_program_address`] with the documented
//! seeds and returns `(address, bump)`.

use solana_pubkey::Pubkey;

/// Seed for the vault authority PDA.
pub const BASE_VAULT_AUTHORITY_SEED: &[u8] = b"authority";
/// Seed for the token vault PDA.
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";
/// Seed for the shares mint PDA.
pub const SHARES_SEED: &[u8] = b"shares";
/// Seed for per-reserve cToken vault PDAs.
pub const CTOKEN_VAULT_SEED: &[u8] = b"ctoken_vault";
/// Seed for the global config PDA.
pub const GLOBAL_CONFIG_SEED: &[u8] = b"global_config";
/// Seed for reserve whitelist entry PDAs.
pub const WHITELISTED_RESERVES_SEED: &[u8] = b"whitelisted_reserves";
/// Seed for the Anchor event authority PDA.
pub const EVENT_AUTHORITY: &[u8] = b"__event_authority";

/// Vault authority PDA that signs CPI calls on behalf of the vault.
///
/// Seeds: `[b"authority", vault_state]`.
pub fn base_vault_authority(program_id: &Pubkey, vault_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[BASE_VAULT_AUTHORITY_SEED, vault_state.as_ref()],
        program_id,
    )
}

/// Token account holding the vault's uninvested liquidity.
///
/// Seeds: `[b"token_vault", vault_state]`.
pub fn token_vault(program_id: &Pubkey, vault_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_VAULT_SEED, vault_state.as_ref()], program_id)
}

/// SPL mint for vault share tokens issued to depositors.
///
/// Seeds: `[b"shares", vault_state]`.
pub fn shares_mint(program_id: &Pubkey, vault_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SHARES_SEED, vault_state.as_ref()], program_id)
}

/// Token account holding cTokens received from a specific Klend reserve.
///
/// Seeds: `[b"ctoken_vault", vault_state, reserve]`.
pub fn ctoken_vault(program_id: &Pubkey, vault_state: &Pubkey, reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CTOKEN_VAULT_SEED, vault_state.as_ref(), reserve.as_ref()],
        program_id,
    )
}

/// System-wide vault configuration account.
///
/// Seeds: `[b"global_config"]`.
pub fn global_config(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], program_id)
}

/// Whitelist entry PDA for a specific Klend reserve.
///
/// Seeds: `[b"whitelisted_reserves", reserve]`.
pub fn whitelisted_reserve(program_id: &Pubkey, reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WHITELISTED_RESERVES_SEED, reserve.as_ref()], program_id)
}

/// Anchor event authority PDA (used for `event_cpi` accounts).
///
/// Seeds: `[b"__event_authority"]`.
pub fn event_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_AUTHORITY], program_id)
}

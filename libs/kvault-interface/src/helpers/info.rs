use std::collections::HashMap;

use solana_pubkey::Pubkey;

use crate::state::{self, VaultState};

/// One active vault allocation: a Klend reserve and the lending market it
/// belongs to.
///
/// The lending market is not stored on the [`VaultState`]; it lives on each
/// reserve account and must be fetched alongside it (see
/// [`VaultInfo::from_vault_state`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultReserve {
    /// Klend reserve address with an active allocation.
    pub reserve: Pubkey,
    /// The lending market this reserve belongs to.
    pub lending_market: Pubkey,
}

/// On-chain vault data that cannot be derived from PDAs.
///
/// The caller reads these fields from the deserialized [`VaultState`] account.
/// Vault PDAs (authority, token vault, shares mint) and remaining accounts
/// are derived automatically by the helpers.
///
/// # Reserve slot-ordering contract
///
/// [`reserves`](Self::reserves) mirrors the on-chain non-empty
/// `vault_allocation_strategy[]` slots **in slot order**. The on-chain refresh
/// check (`check_allocation_reserves_refreshed`) walks the refresh remaining
/// accounts in exactly that order, so the order must be preserved. The
/// constructors enforce this by re-deriving the order from the allocation
/// strategy regardless of the order in which reserve data is supplied, so the
/// refresh check cannot be violated through this type.
pub struct VaultInfo {
    /// Vault account address.
    pub address: Pubkey,
    /// SPL token mint for the vault's underlying asset (e.g. USDC).
    pub token_mint: Pubkey,
    /// Token program for [`token_mint`](Self::token_mint) (`TOKEN_PROGRAM_ID` or Token-2022).
    pub token_program: Pubkey,
    /// Active allocations (reserve + lending market) in `vault_allocation_strategy[]` slot order.
    reserves: Vec<VaultReserve>,
}

/// Errors returned when building a [`VaultInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultInfoError {
    /// Failed to deserialize the vault account data.
    AccountData(state::AccountDataError),
    /// An active allocation reserve had no matching [`ReserveInfo`] provided.
    MissingReserve(Pubkey),
}

impl core::fmt::Display for VaultInfoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AccountData(err) => write!(f, "{err}"),
            Self::MissingReserve(reserve) => {
                write!(f, "no ReserveInfo provided for active reserve {reserve}")
            }
        }
    }
}

impl std::error::Error for VaultInfoError {}

impl From<state::AccountDataError> for VaultInfoError {
    fn from(err: state::AccountDataError) -> Self {
        Self::AccountData(err)
    }
}

impl VaultInfo {
    /// Addresses of the reserves with active allocations, in
    /// `vault_allocation_strategy[]` slot order.
    ///
    /// Use this to learn which reserve accounts to fetch *before* building a
    /// [`VaultInfo`] (the lending markets needed by the constructors live on
    /// those reserve accounts).
    pub fn active_reserve_addresses(vault: &VaultState) -> Vec<Pubkey> {
        vault
            .vault_allocation_strategy
            .iter()
            .map(|a| a.reserve)
            .filter(|r| r != &Pubkey::default())
            .collect()
    }

    /// Build from a deserialized [`VaultState`] and the fetched reserve data.
    ///
    /// The reserve+lending-market list is built by iterating the active
    /// allocations **in slot order** and looking up the matching [`ReserveInfo`]
    /// by address, so the order of `reserves` does not matter. Returns
    /// [`VaultInfoError::MissingReserve`] if any active allocation reserve has
    /// no matching [`ReserveInfo`].
    pub fn from_vault_state(
        address: Pubkey,
        vault: &VaultState,
        reserves: &[ReserveInfo],
    ) -> Result<Self, VaultInfoError> {
        let lending_markets: HashMap<Pubkey, Pubkey> = reserves
            .iter()
            .map(|r| (r.address, r.lending_market))
            .collect();

        let reserves = Self::active_reserve_addresses(vault)
            .into_iter()
            .map(|reserve| {
                let lending_market = lending_markets
                    .get(&reserve)
                    .copied()
                    .ok_or(VaultInfoError::MissingReserve(reserve))?;
                Ok(VaultReserve {
                    reserve,
                    lending_market,
                })
            })
            .collect::<Result<Vec<_>, VaultInfoError>>()?;

        Ok(Self {
            address,
            token_mint: vault.token_mint,
            token_program: vault.token_program,
            reserves,
        })
    }

    /// Build from raw account bytes (includes 8-byte discriminator) and the
    /// fetched reserve data.
    ///
    /// See [`from_vault_state`](Self::from_vault_state) for the slot-ordering
    /// contract and the `reserves` lookup semantics.
    pub fn from_account_data(
        address: Pubkey,
        data: &[u8],
        reserves: &[ReserveInfo],
    ) -> Result<Self, VaultInfoError> {
        let vault = state::from_account_data::<VaultState>(data)?;
        Self::from_vault_state(address, vault, reserves)
    }

    /// Active allocations (reserve + lending market) in
    /// `vault_allocation_strategy[]` slot order.
    ///
    /// The order mirrors the on-chain non-empty allocation slots and is
    /// enforced by the constructors (see the [type-level
    /// docs](VaultInfo#reserve-slot-ordering-contract)), so it is safe to use
    /// directly to build the refresh remaining accounts.
    pub fn reserves(&self) -> &[VaultReserve] {
        &self.reserves
    }

    /// Addresses of the reserves with active allocations, in slot order.
    ///
    /// Convenience iterator for read-only consumers that only need the reserve
    /// pubkeys.
    pub fn reserve_addresses(&self) -> impl Iterator<Item = Pubkey> + '_ {
        self.reserves.iter().map(|r| r.reserve)
    }
}

/// On-chain Klend reserve data needed by helpers that interact with reserves
/// (withdraw, invest, redeem).
///
/// The caller reads these fields from the deserialized
/// [`Reserve`](klend_interface::state::Reserve) account. Reserve PDAs are
/// used directly (not re-derived) because they are stored on the reserve itself.
pub struct ReserveInfo {
    /// Reserve account address.
    pub address: Pubkey,
    /// The lending market this reserve belongs to.
    pub lending_market: Pubkey,
    /// Token account holding the reserve's available liquidity.
    pub liquidity_supply_vault: Pubkey,
    /// SPL mint for the reserve's collateral tokens (cTokens).
    pub collateral_mint: Pubkey,
}

impl ReserveInfo {
    /// Build from a deserialized klend [`Reserve`](klend_interface::state::Reserve).
    pub fn from_reserve(address: Pubkey, reserve: &klend_interface::state::Reserve) -> Self {
        Self {
            address,
            lending_market: reserve.lending_market,
            liquidity_supply_vault: reserve.liquidity.supply_vault,
            collateral_mint: reserve.collateral.mint_pubkey,
        }
    }

    /// Build from raw klend reserve account bytes (includes 8-byte discriminator).
    pub fn from_account_data(
        address: Pubkey,
        data: &[u8],
    ) -> Result<Self, klend_interface::state::AccountDataError> {
        let reserve =
            klend_interface::state::from_account_data::<klend_interface::state::Reserve>(data)?;
        Ok(Self::from_reserve(address, reserve))
    }
}

impl From<(Pubkey, &klend_interface::state::Reserve)> for ReserveInfo {
    fn from((address, reserve): (Pubkey, &klend_interface::state::Reserve)) -> Self {
        Self::from_reserve(address, reserve)
    }
}

impl TryFrom<(Pubkey, &[u8])> for ReserveInfo {
    type Error = klend_interface::state::AccountDataError;
    fn try_from((address, data): (Pubkey, &[u8])) -> Result<Self, Self::Error> {
        Self::from_account_data(address, data)
    }
}

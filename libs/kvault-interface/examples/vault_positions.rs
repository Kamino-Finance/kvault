//! Enumerate **every** position in a vault: unstaked share balances plus farm stakes.
//!
//! Kvault has no per-user account — `getProgramAccounts` against the Kvault program will
//! never return positions. A position is the sum of two things owned by *other* programs:
//!
//! 1. Shares held in an SPL token account of the vault's shares mint.
//! 2. Shares staked in the vault's Kamino Farms farm (`VaultState::vault_farm`, and the
//!    separate `first_loss_capital_farm`).
//!
//! So enumeration is two `getProgramAccounts` scans, joined by owner:
//!
//! ```text
//! A) program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
//!    filters dataSize 165, memcmp{0, shares_mint}
//!
//! B) program FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr
//!    filters dataSize 920, memcmp{16, farm_state}     // UserState.farm_state
//! ```
//!
//! This handles all three vault shapes: no farm, farm with some shares unstaked, and farm
//! with every share staked.
//!
//! # Three things that make a naive version wrong
//!
//! 1. **`active_stake_scaled` is a stake *share*, not a token amount.** Dividing by WAD is
//!    wrong. Kamino Farms uses a two-pool share model so that slashing can move the
//!    stake:amount ratio, so the amount must be recovered with [`convert_stake_to_amount`].
//! 2. **Pending stake converts against a different pool** (`total_pending_stake_scaled` /
//!    `total_pending_amount`) than active stake (`total_active_stake_scaled` /
//!    `total_staked_amount`). Both pending buckets still belong to the user: warmup shares
//!    are already deposited, and cooldown shares stay in the farm vault until
//!    `withdraw_unstaked_deposits` runs.
//! 3. **The farm vault is itself a shares token account** and shows up in scan A holding
//!    *all* staked shares. It must be excluded, or every staked share is counted twice.
//!
//! The reconciliation line printed at the end is what catches all three: the positions must
//! sum to the circulating supply of the shares mint, leaving only floor-rounding dust.
//!
//! Reconcile against the mint supply, *not* `VaultState::shares_issued` — that field is
//! Kvault's internal counter and can sit above the real supply, because a holder can burn
//! shares directly through the SPL Token program without going through Kvault. It remains
//! the right denominator for valuing a position.
//!
//! # Caveats
//!
//! - `getProgramAccounts` with a mint filter is a heavy call and some public RPC endpoints
//!   disable or throttle it. `getTokenLargestAccounts(shares_mint)` is the cheap fallback,
//!   but returns only the top 20 holders.
//! - Results include zero-balance token accounts and holders that are not end-user wallets
//!   (AMM pools, aggregators, other protocols). Those are real holders, so they are listed
//!   rather than dropped — but they are not retail positions.
//! - Scan B fetches whole 920-byte accounts. For a farm with very many users, narrowing with
//!   a `dataSlice` and hand-decoding is faster, at the cost of the layout checks below.
//! - The scans are separate RPC calls, so `minContextSlot` is pinned across all of them;
//!   without it a stake landing mid-sequence makes the reconciliation fail spuriously.
//!
//! ```text
//! cargo run --example vault_positions -- [VAULT_PUBKEY]
//! RPC_URL=https://... cargo run --example vault_positions
//! ```

use std::{collections::HashMap, str::FromStr};

use bytemuck::{Pod, Zeroable};
use kvault_interface::{
    from_account_data, pda,
    state::{PodU128, VaultState},
    Fraction, KVAULT_PROGRAM_ID, TOKEN_PROGRAM_ID,
};
use solana_account_decoder_client_types::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_pubkey::{pubkey, Pubkey};
use spl_discriminator::SplDiscriminate;
use uint::construct_uint;

/// Kamino Farms program ID.
const KFARMS_PROGRAM_ID: Pubkey = pubkey!("FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr");

/// Size of an SPL token account (classic SPL Token, no Token-2022 extensions).
const SPL_TOKEN_ACCOUNT_LEN: u64 = 165;

/// Default vault, matching the other examples.
const DEFAULT_VAULT: &str = "HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E";

/// How many positions to print before truncating. Totals always cover every position.
const TOP_N: usize = 25;

/// 256-bit intermediate for the stake -> amount conversion.
///
/// Scoped into a module so the lint allows cover only the generated code.
#[allow(clippy::manual_div_ceil, clippy::assign_op_pattern)]
mod u256 {
    use super::construct_uint;
    construct_uint! {
        pub struct U256(4);
    }
}
use u256::U256;

// --- Kamino Farms account mirrors -----------------------------------------------------------
//
// Mirrors of the two `kfarms` accounts, kept local to this example so the library gains no
// dependency on the farms program. Fields this example never reads are collapsed into opaque
// byte blobs. The `const _: () = assert!(..)` blocks below pin every size and offset the code
// relies on, so a `kfarms` layout change breaks the build instead of silently producing wrong
// numbers.
//
// Layout source: https://github.com/Kamino-Finance/kfarms `programs/kfarms/src/state.rs`
// (`FarmState`, `UserState`), at the revision pinned in the workspace `Cargo.lock`.
// `u128` is align-8 on the SBF target, so neither struct has implicit padding.

/// Kamino Farms `UserState` — one per (farm, delegatee). 912 bytes + 8 discriminator.
#[allow(dead_code)]
#[derive(Clone, Copy, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("account:UserState")]
#[repr(C)]
struct FarmUserState {
    user_id: u64,
    /// The farm this stake belongs to — the `memcmp` target of scan B.
    farm_state: Pubkey,
    /// Wallet the position belongs to.
    owner: Pubkey,
    is_farm_delegated: u8,
    _padding_0: [u8; 7],
    _rewards_tally_scaled: [u8; 160],
    _rewards_issued_unclaimed: [u8; 80],
    _last_claim_ts: [u8; 80],
    /// Active stake *share* — not a token amount. See [`convert_stake_to_amount`].
    active_stake_scaled: PodU128,
    /// Stake share in deposit warmup.
    pending_deposit_stake_scaled: PodU128,
    pending_deposit_stake_ts: u64,
    /// Stake share in withdrawal cooldown; still held by the farm vault.
    pending_withdrawal_unstake_scaled: PodU128,
    pending_withdrawal_unstake_ts: u64,
    bump: u64,
    delegatee: Pubkey,
    last_stake_ts: u64,
    _padding_1: [u8; 400],
}

const _: () = assert!(core::mem::size_of::<FarmUserState>() == 912);
const _: () = assert!(core::mem::offset_of!(FarmUserState, farm_state) == 8);
const _: () = assert!(core::mem::offset_of!(FarmUserState, owner) == 40);
const _: () = assert!(core::mem::offset_of!(FarmUserState, active_stake_scaled) == 400);
const _: () = assert!(core::mem::offset_of!(FarmUserState, pending_deposit_stake_scaled) == 416);
const _: () = assert!(core::mem::offset_of!(FarmUserState, pending_withdrawal_unstake_scaled) == 440);

/// Account-data offset of `UserState::farm_state` (struct offset + 8-byte discriminator).
const USER_STATE_FARM_OFFSET: usize = core::mem::offset_of!(FarmUserState, farm_state) + 8;
/// Full on-chain size of a `UserState` account (`kfarms` `SIZE_USER_STATE`).
const USER_STATE_LEN: u64 = core::mem::size_of::<FarmUserState>() as u64 + 8;

/// Kamino Farms `FarmState`. 8328 bytes + 8 discriminator.
#[allow(dead_code)]
#[derive(Clone, Copy, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("account:FarmState")]
#[repr(C)]
struct FarmState {
    _farm_admin: Pubkey,
    _global_config: Pubkey,
    /// Mint being staked — for a vault farm this is the vault's shares mint.
    token_mint: Pubkey,
    _token_rest: [u8; 88],
    _reward_infos: [u8; 7040],
    _num_reward_tokens: u64,
    /// Number of `UserState` accounts the farm has created.
    num_users: u64,
    /// Token amount backing the *active* stake pool.
    total_staked_amount: u64,
    /// Token account holding every staked share — must be excluded from scan A.
    farm_vault: Pubkey,
    _mid: [u8; 120],
    /// Total active stake shares.
    total_active_stake_scaled: PodU128,
    /// Total pending stake shares (deposit warmup *and* withdrawal cooldown).
    total_pending_stake_scaled: PodU128,
    /// Token amount backing the pending stake pool.
    total_pending_amount: u64,
    _tail: [u8; 888],
}

const _: () = assert!(core::mem::size_of::<FarmState>() == 8328);
const _: () = assert!(core::mem::offset_of!(FarmState, token_mint) == 64);
const _: () = assert!(core::mem::offset_of!(FarmState, num_users) == 7232);
const _: () = assert!(core::mem::offset_of!(FarmState, total_staked_amount) == 7240);
const _: () = assert!(core::mem::offset_of!(FarmState, farm_vault) == 7248);
const _: () = assert!(core::mem::offset_of!(FarmState, total_active_stake_scaled) == 7400);
const _: () = assert!(core::mem::offset_of!(FarmState, total_pending_stake_scaled) == 7416);
const _: () = assert!(core::mem::offset_of!(FarmState, total_pending_amount) == 7432);

// --- Stake -> amount ------------------------------------------------------------------------

/// Convert a stake share into a token amount, mirroring `kfarms`
/// `stake_operations::convert_stake_to_amount` with `round_up: false`.
///
/// `amount = floor(stake * total_amount / total_stake)`.
///
/// On-chain this runs through `full_decimal_mul_div`, which multiplies by `WAD` and divides it
/// back out at the final `try_floor`. `floor(floor(x / a) / b) == floor(x / (a * b))` for
/// non-negative integers, so the simpler form here is exactly equal.
///
/// The 256-bit intermediate is not optional: `stake` is already WAD-scaled (~2^110 for
/// realistic balances) and `total_amount` reaches ~2^50, so the product overflows `u128`.
fn convert_stake_to_amount(stake_scaled: u128, total_stake_scaled: u128, total_amount: u64) -> u64 {
    if stake_scaled == 0 {
        return 0;
    }
    if total_stake_scaled == 0 {
        return total_amount;
    }
    let quotient =
        U256::from(stake_scaled) * U256::from(total_amount) / U256::from(total_stake_scaled);

    // A consistent farm has stake <= total_stake, so this always fits; saturate rather than
    // panic if an RPC hands back a torn read.
    if quotient > U256::from(u64::MAX) {
        u64::MAX
    } else {
        quotient.as_u64()
    }
}

/// Shares a user holds in a farm, as `(active, pending)` token amounts.
///
/// Active and pending convert against *different* pools; `total_staked_amount` backs the active
/// pool only, so the two never double-count each other.
fn staked_shares(user: &FarmUserState, farm: &FarmState) -> (u64, u64) {
    let active = convert_stake_to_amount(
        user.active_stake_scaled.into(),
        farm.total_active_stake_scaled.into(),
        farm.total_staked_amount,
    );

    let total_pending_stake = farm.total_pending_stake_scaled.into();
    let pending_deposit = convert_stake_to_amount(
        user.pending_deposit_stake_scaled.into(),
        total_pending_stake,
        farm.total_pending_amount,
    );
    let pending_withdrawal = convert_stake_to_amount(
        user.pending_withdrawal_unstake_scaled.into(),
        total_pending_stake,
        farm.total_pending_amount,
    );

    (active, pending_deposit.saturating_add(pending_withdrawal))
}

// --- Position accumulation ------------------------------------------------------------------

#[derive(Default, Clone, Copy)]
struct Position {
    unstaked: u64,
    staked_active: u64,
    staked_pending: u64,
}

impl Position {
    fn total(&self) -> u64 {
        self.unstaked
            .saturating_add(self.staked_active)
            .saturating_add(self.staked_pending)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vault_pubkey = Pubkey::from_str(
        &std::env::args()
            .nth(1)
            .unwrap_or_else(|| DEFAULT_VAULT.to_string()),
    )?;
    let rpc_url =
        std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
    let rpc_client = RpcClient::new(rpc_url);

    // Pin every read to one slot so the reconciliation below cannot be broken by a stake
    // landing between the scans.
    let slot = rpc_client.get_slot()?;

    // --- 1. Vault state -----------------------------------------------------------------

    let vault_account = rpc_client.get_account(&vault_pubkey)?;
    let vault = from_account_data::<VaultState>(&vault_account.data)?;
    let (shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_pubkey);

    // The circulating supply is what the position scans must add up to. SPL mint layout:
    // mint_authority COption<Pubkey> 0..36, supply u64 36..44.
    let mint_account = rpc_client.get_account(&shares_mint)?;
    let shares_supply = u64::from_le_bytes(mint_account.data[36..44].try_into()?);

    println!("Vault: {vault_pubkey}");
    println!("  Shares mint: {shares_mint}");
    println!("  Shares supply: {shares_supply}");
    println!("  Shares issued (vault counter): {}", vault.shares_issued);
    println!("  Slot: {slot}");

    let mut positions: HashMap<Pubkey, Position> = HashMap::new();

    // --- 2. Scan A: unstaked shares (SPL Token program) ---------------------------------

    let share_accounts = rpc_client.get_program_accounts_with_config(
        &TOKEN_PROGRAM_ID,
        RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::DataSize(SPL_TOKEN_ACCOUNT_LEN),
                RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, shares_mint.as_ref())),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                // Only owner (32) + amount (8) are needed, at token account offset 32.
                data_slice: Some(UiDataSliceConfig {
                    offset: 32,
                    length: 40,
                }),
                min_context_slot: Some(slot),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )?;

    // --- 3. Scan B: staked shares (Kamino Farms), per farm ------------------------------

    let farms: Vec<(&str, Pubkey)> = [
        ("vault_farm", vault.vault_farm),
        ("first_loss_capital_farm", vault.first_loss_capital_farm),
    ]
    .into_iter()
    .filter(|(_, farm)| *farm != Pubkey::default())
    .collect();

    // Token accounts owned by a farm hold *everyone's* staked shares, so they must not be
    // counted as positions in scan A.
    let mut farm_vaults: Vec<Pubkey> = Vec::new();
    let mut farm_pool_totals: u64 = 0;
    // Each staker's amount is floored independently, so this bounds the acceptable dust.
    let mut staker_count: u128 = 0;

    if farms.is_empty() {
        println!("\nNo farm configured — all positions are held as share token accounts.");
    }

    for (label, farm_pubkey) in &farms {
        let farm_account = rpc_client.get_account(farm_pubkey)?;
        let farm = from_account_data::<FarmState>(&farm_account.data)?;

        if farm.token_mint != shares_mint {
            return Err(format!(
                "{label} {farm_pubkey} stakes {}, not the vault shares mint {shares_mint}",
                farm.token_mint
            )
            .into());
        }
        farm_vaults.push(farm.farm_vault);
        farm_pool_totals = farm_pool_totals
            .saturating_add(farm.total_staked_amount)
            .saturating_add(farm.total_pending_amount);

        let user_states = rpc_client.get_program_accounts_with_config(
            &KFARMS_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(vec![
                    RpcFilterType::DataSize(USER_STATE_LEN),
                    RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                        USER_STATE_FARM_OFFSET,
                        farm_pubkey.as_ref(),
                    )),
                ]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    min_context_slot: Some(slot),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )?;

        println!("\n{label}: {farm_pubkey}");
        println!("  Farm vault: {} (excluded from holders)", farm.farm_vault);
        println!("  Active pool: {} tokens", farm.total_staked_amount);
        println!("  Pending pool: {} tokens", farm.total_pending_amount);
        println!(
            "  User states: {} returned, num_users = {}",
            user_states.len(),
            farm.num_users
        );

        staker_count += user_states.len() as u128;
        for (_, account) in &user_states {
            let user = from_account_data::<FarmUserState>(&account.data)?;
            let (active, pending) = staked_shares(user, farm);
            let entry = positions.entry(user.owner).or_default();
            entry.staked_active = entry.staked_active.saturating_add(active);
            entry.staked_pending = entry.staked_pending.saturating_add(pending);
        }
    }

    // --- 4. Merge scan A, excluding the farm vaults --------------------------------------

    let mut excluded_from_farm_vaults: u64 = 0;
    for (address, account) in &share_accounts {
        let owner = Pubkey::try_from(&account.data[..32])?;
        let amount = u64::from_le_bytes(account.data[32..40].try_into()?);

        if farm_vaults.contains(address) {
            excluded_from_farm_vaults = excluded_from_farm_vaults.saturating_add(amount);
            continue;
        }
        positions.entry(owner).or_default().unstaked += amount;
    }

    // --- 5. Report ------------------------------------------------------------------------

    let mut ranked: Vec<(Pubkey, Position)> = positions
        .into_iter()
        .filter(|(_, p)| p.total() > 0)
        .collect();
    ranked.sort_by(|a, b| b.1.total().cmp(&a.1.total()));

    let total_unstaked: u128 = ranked.iter().map(|(_, p)| u128::from(p.unstaked)).sum();
    let total_staked: u128 = ranked
        .iter()
        .map(|(_, p)| u128::from(p.staked_active) + u128::from(p.staked_pending))
        .sum();
    let total_shares = total_unstaked + total_staked;

    let aum = Fraction::from_bits(u128::from(vault.prev_aum_sf));
    let exchange_rate = if vault.shares_issued > 0 {
        aum / Fraction::from_num(vault.shares_issued)
    } else {
        Fraction::from_num(0)
    };

    println!("\nPositions: {}", ranked.len());
    println!(
        "{:<44} {:>18} {:>18} {:>18} {:>18}",
        "Owner", "Unstaked", "Staked", "Total shares", "Value (tokens)"
    );
    for (owner, position) in ranked.iter().take(TOP_N) {
        let staked = u128::from(position.staked_active) + u128::from(position.staked_pending);
        let value = Fraction::from_num(position.total()) * exchange_rate;
        println!(
            "{:<44} {:>18} {:>18} {:>18} {:>18.6}",
            owner.to_string(),
            position.unstaked,
            staked,
            position.total(),
            value
        );
    }
    if ranked.len() > TOP_N {
        println!("... {} more (totals below cover all)", ranked.len() - TOP_N);
    }

    // --- 6. Reconciliation ----------------------------------------------------------------
    //
    // The positions must add up to the *circulating supply of the shares mint*. That is the
    // check that catches a missing farm scan, a wrong stake conversion, or a farm vault
    // counted as a holder. The only legitimate gap is floor rounding in the stake
    // conversion, bounded by the number of stakers.
    //
    // Deliberately not `VaultState::shares_issued`: that is Kvault's internal accounting
    // counter, and it can drift above the mint supply (holders can burn shares directly
    // through the SPL Token program without going through Kvault, which lowers the supply
    // but not the counter). It stays the right denominator for *valuing* a position, which
    // is what the exchange rate above uses.

    let delta = i128::from(shares_supply) - total_shares as i128;

    println!("\nReconciliation");
    println!("  Unstaked: {total_unstaked}");
    println!("  Staked:   {total_staked}");
    println!("  Total:    {total_shares}");
    println!("  Shares supply: {shares_supply}");
    if delta >= 0 && delta as u128 <= staker_count {
        println!("  Delta vs supply: {delta}  (floor-rounding dust, <= 1 per staker)");
    } else {
        println!(
            "  Delta vs supply: {delta}  MISMATCH \u{2014} exceeds the {staker_count}-share dust bound"
        );
    }

    if !farm_vaults.is_empty() {
        println!(
            "  Farm vaults held {excluded_from_farm_vaults} shares, farm pools account for \
             {farm_pool_totals} (excluded from holders to avoid double counting)"
        );
    }

    let counter_gap = i128::from(vault.shares_issued) - i128::from(shares_supply);
    if counter_gap != 0 {
        println!("  Note: vault shares_issued exceeds mint supply by {counter_gap}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAD: u128 = 1_000_000_000_000_000_000;

    #[test]
    fn zero_stake_is_zero() {
        assert_eq!(convert_stake_to_amount(0, 100 * WAD, 100), 0);
    }

    #[test]
    fn empty_pool_returns_total_amount() {
        // Mirrors the `total_stake == Decimal::zero()` branch in kfarms.
        assert_eq!(convert_stake_to_amount(5 * WAD, 0, 42), 42);
    }

    #[test]
    fn one_to_one_pool() {
        // Untouched farm: stake and amount move together.
        assert_eq!(convert_stake_to_amount(25 * WAD, 100 * WAD, 100), 25);
    }

    #[test]
    fn slashed_pool_is_not_one_to_one() {
        // 100 stake shares now back only 90 tokens. Dividing the stake by WAD would say 25.
        assert_eq!(convert_stake_to_amount(25 * WAD, 100 * WAD, 90), 22);
    }

    #[test]
    fn rounds_down() {
        // 1/3 of 100 tokens floors to 33, matching `round_up: false` on-chain.
        assert_eq!(convert_stake_to_amount(WAD, 3 * WAD, 100), 33);
    }

    #[test]
    fn realistic_magnitudes_do_not_overflow_u128() {
        // 10M shares at 9 decimals, WAD-scaled: the product with total_amount is ~2^163,
        // so a u128 intermediate would wrap. This is the case that passes toy tests and
        // fails in production.
        let total_amount: u64 = 10_000_000_000_000_000; // 1e16
        let total_stake = u128::from(total_amount) * WAD;
        let user_stake = total_stake / 4;

        assert!(u128::from(total_amount).checked_mul(user_stake).is_none());
        assert_eq!(
            convert_stake_to_amount(user_stake, total_stake, total_amount),
            total_amount / 4
        );
    }

    #[test]
    fn full_stake_recovers_full_amount() {
        let total_amount: u64 = 123_456_789;
        let total_stake = 987_654_321_u128 * WAD;
        assert_eq!(
            convert_stake_to_amount(total_stake, total_stake, total_amount),
            total_amount
        );
    }
}

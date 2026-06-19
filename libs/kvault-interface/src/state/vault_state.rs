use bytemuck::{Pod, Zeroable};
use solana_pubkey::Pubkey;
use spl_discriminator::SplDiscriminate;

use super::pod::PodU128;

use super::{VaultAllocation, VaultRewardInfo};
use crate::MAX_RESERVES;

/// Kamino Vault account state.
#[derive(Debug, Clone, Copy, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("account:VaultState")]
#[repr(C)]
pub struct VaultState {
    /// Wallet authorised to manage the vault (update config, allocations, etc.).
    pub vault_admin_authority: Pubkey,

    /// PDA that signs CPI calls on behalf of the vault.
    pub base_vault_authority: Pubkey,
    /// Bump seed for [`base_vault_authority`](Self::base_vault_authority).
    pub base_vault_authority_bump: u64,

    /// SPL token mint for the vault's underlying asset (e.g. USDC).
    pub token_mint: Pubkey,
    /// Decimal places of [`token_mint`](Self::token_mint).
    pub token_mint_decimals: u64,
    /// Token account holding the vault's uninvested (available) liquidity.
    pub token_vault: Pubkey,
    /// Token program for [`token_mint`](Self::token_mint) (`TOKEN_PROGRAM_ID` or Token-2022).
    pub token_program: Pubkey,

    /// SPL mint for vault share tokens issued to depositors.
    pub shares_mint: Pubkey,
    /// Decimal places of [`shares_mint`](Self::shares_mint).
    pub shares_mint_decimals: u64,

    /// Amount of underlying tokens currently available (not invested).
    pub token_available: u64,
    /// Total vault share tokens currently outstanding.
    pub shares_issued: u64,

    /// Tokens reserved for crank operation fees.
    pub available_crank_funds: u64,
    /// Weight assigned to the unallocated (idle) portion of the vault.
    pub unallocated_weight: u64,

    /// Performance fee rate in basis points.
    pub performance_fee_bps: u64,
    /// Annual management fee rate in basis points.
    pub management_fee_bps: u64,
    /// Unix timestamp of the last fee charge.
    pub last_fee_charge_timestamp: u64,
    /// Previous assets under management as a scaled fraction ([`Fraction`](crate::Fraction)).
    pub prev_aum_sf: PodU128,
    /// Accrued but uncollected fees as a scaled fraction ([`Fraction`](crate::Fraction)).
    pub pending_fees_sf: PodU128,

    /// Per-reserve allocation slots. Active reserves have a non-default `reserve` pubkey.
    pub vault_allocation_strategy: [VaultAllocation; MAX_RESERVES],
    /// Reserved for future use.
    pub padding_1: [PodU128; 256],

    /// Minimum token amount accepted for a deposit.
    pub min_deposit_amount: u64,
    /// Minimum share amount accepted for a withdrawal.
    pub min_withdraw_amount: u64,
    /// Minimum token amount for an invest operation.
    pub min_invest_amount: u64,
    /// Minimum number of slots between consecutive invest calls.
    pub min_invest_delay_slots: u64,
    /// Crank fee charged per reserve during invest operations.
    pub crank_fund_fee_per_reserve: u64,

    /// Wallet that has been nominated as the next admin (two-step transfer).
    pub pending_admin: Pubkey,

    /// Cumulative interest earned by the vault, scaled fraction.
    pub cumulative_earned_interest_sf: PodU128,
    /// Cumulative management fees collected, scaled fraction.
    pub cumulative_mgmt_fees_sf: PodU128,
    /// Cumulative performance fees collected, scaled fraction.
    pub cumulative_perf_fees_sf: PodU128,

    /// Human-readable vault name (UTF-8, up to 40 bytes).
    pub name: [u8; 40],
    /// Address lookup table for the vault's accounts.
    pub vault_lookup_table: Pubkey,
    /// Kamino Farms state for the vault's share token farm.
    pub vault_farm: Pubkey,

    /// Unix timestamp when the vault was created.
    pub creation_timestamp: u64,

    /// Maximum amount of tokens that can remain unallocated.
    pub unallocated_tokens_cap: u64,
    /// Wallet authorised to manage allocations (may differ from [`vault_admin_authority`](Self::vault_admin_authority)).
    pub allocation_admin: Pubkey,

    /// Flat withdrawal penalty in lamports.
    pub withdrawal_penalty_lamports: u64,
    /// Withdrawal penalty in basis points.
    pub withdrawal_penalty_bps: u64,

    /// Farm state for first-loss capital providers.
    pub first_loss_capital_farm: Pubkey,

    /// When non-zero, allocations are restricted to whitelisted reserves only.
    pub allow_allocations_in_whitelisted_reserves_only: u8,
    /// When non-zero, invest is restricted to whitelisted reserves only.
    pub allow_invest_in_whitelisted_reserves_only: u8,

    /// Reserved for future use.
    pub padding_2: [u8; 6],
    /// Total vault deposit cap; `0` means uncapped (for backward compatibility).
    /// This is a soft cap that only blocks new deposits — the vault AUM can still
    /// grow above it through earned interest.
    pub deposit_cap: u64,
    /// Reward distribution state ([`VaultRewardInfo`]).
    pub reward_info: VaultRewardInfo,
    /// Reserved for future use.
    pub padding_3: [PodU128; 232],
}

const _: () = assert!(core::mem::size_of::<VaultState>() == 62544);

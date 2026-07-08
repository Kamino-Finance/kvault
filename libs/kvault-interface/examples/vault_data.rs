//! Fetch and inspect vault state, allocations, and AUM.
//!
//! Demonstrates how to:
//! - Parse a `VaultState` account
//! - Read vault configuration (fees, min amounts)
//! - Inspect per-reserve allocations and cToken balances
//!
//! Fields suffixed with `_sf` are scaled fractions — use
//! `Fraction::from_bits(u128_value)` to convert.
//!
//! ```text
//! cargo run --example vault_data
//! ```

use std::str::FromStr;

use kvault_interface::{
    state::{VaultAllocation, VaultState},
    Fraction,
};
use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com");
    let vault_pubkey = Pubkey::from_str("HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E")?;

    // --- 1. Fetch and parse the vault -------------------------------------------------------

    let vault_account = rpc_client.get_account(&vault_pubkey)?;
    let vault = kvault_interface::from_account_data::<VaultState>(&vault_account.data)?;

    println!("Vault: {vault_pubkey}");
    println!("  Admin: {}", vault.vault_admin_authority);
    println!("  Token mint: {}", vault.token_mint);
    println!("  Available liquidity: {}", vault.token_available);
    println!("  Shares issued: {}", vault.shares_issued);
    println!("  Performance fee: {} bps", vault.performance_fee_bps);
    println!("  Management fee: {} bps", vault.management_fee_bps);

    let prev_aum = Fraction::from_bits(u128::from(vault.prev_aum_sf));
    println!("  Previous AUM: {prev_aum:.6}");

    // --- 2. Inspect allocations -------------------------------------------------------------

    let active_allocations: Vec<&VaultAllocation> = vault
        .vault_allocation_strategy
        .iter()
        .filter(|a| a.reserve != Pubkey::default())
        .collect();

    println!("\n  Active allocations: {}", active_allocations.len());
    for alloc in &active_allocations {
        let target = Fraction::from_bits(u128::from(alloc.token_target_allocation_sf));
        println!("    Reserve: {}", alloc.reserve);
        println!("      Weight: {}", alloc.target_allocation_weight);
        println!("      cToken balance: {}", alloc.ctoken_allocation);
        println!("      Token cap: {}", alloc.token_allocation_cap);
        println!("      CToken cap: {}", alloc.ctoken_allocation_cap);
        println!("      Target allocation: {target:.6}");
    }

    Ok(())
}

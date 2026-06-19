//! Read a user's share balance and compute token value.
//!
//! Demonstrates how to:
//! - Look up a user's vault share token account
//! - Compute the exchange rate from vault AUM and shares issued
//! - Estimate the user's position in underlying tokens
//!
//! NOTE: this only counts shares held in the user's share ATA. If the vault
//! has a share farm (`VaultState::vault_farm` is set), most users keep their
//! shares staked in the farm rather than in the ATA, so this example
//! undercounts their position. A complete implementation must also read the
//! user's staked balance from the vault's farm state (Kamino Farms) and add it
//! to the ATA balance.
//!
//! ```text
//! cargo run --example user_position
//! ```

use std::str::FromStr;

use kvault_interface::{pda, state::VaultState, Fraction, KVAULT_PROGRAM_ID};
use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com");

    let vault_pubkey = Pubkey::from_str("HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E")?;
    let user = Pubkey::from_str("EZC9wzVCvihCsCHEMGADYdsRhcpdRYWzSCZAVegSCfqY")?;

    // --- 1. Fetch vault state ---------------------------------------------------------------

    let vault_account = rpc_client.get_account(&vault_pubkey)?;
    let vault = kvault_interface::from_account_data::<VaultState>(&vault_account.data)?;

    // --- 2. Fetch user's share token balance ------------------------------------------------

    let (shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_pubkey);
    let user_shares_ata = get_associated_token_address(&user, &shares_mint);

    let shares_account = rpc_client.get_account(&user_shares_ata)?;
    // SPL token account: amount is at bytes 64..72
    // NOTE: shares staked in the vault's farm (if any) are NOT counted here —
    // see the module-level note. To get the full position, also read the
    // user's staked balance from `vault.vault_farm` and add it.
    let user_shares = u64::from_le_bytes(shares_account.data[64..72].try_into()?);

    // --- 3. Compute exchange rate and position value -----------------------------------------

    let aum = Fraction::from_bits(u128::from(vault.prev_aum_sf));
    let total_shares = vault.shares_issued;

    println!("Vault: {vault_pubkey}");
    println!("  Total AUM: {aum:.6}");
    println!("  Shares issued: {total_shares}");
    println!("\nUser: {user}");
    println!("  Shares held: {user_shares}");

    if total_shares > 0 {
        let exchange_rate = aum / Fraction::from_num(total_shares);
        let position_value = Fraction::from_num(user_shares) * exchange_rate;
        println!("  Exchange rate: {exchange_rate:.6} tokens/share");
        println!("  Position value: {position_value:.6} tokens");
    } else {
        println!("  Vault has no shares issued");
    }

    Ok(())
}

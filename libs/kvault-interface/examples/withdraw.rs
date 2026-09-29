//! Withdraw shares from a vault to receive the underlying tokens.
//!
//! Demonstrates a full withdrawal that may disinvest from a Klend reserve
//! if the vault's available balance is insufficient.
//!
//! NOTE: if the vault has a share farm (`VaultState::vault_farm` is set), the
//! user's shares are staked in the farm and must be unstaked first — the
//! withdraw instruction can only burn shares held in the user's share ATA. A
//! complete flow prepends a Kamino Farms unstake instruction before the
//! withdraw.
//!
//! ```text
//! cargo run --example withdraw
//! ```

use std::str::FromStr;

use kvault_interface::{
    from_account_data, helpers, pda, state::VaultState, ReserveInfo, VaultInfo, KVAULT_PROGRAM_ID,
};
use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use solana_sdk::signer::{keypair::read_keypair_file, Signer};
use spl_associated_token_account::get_associated_token_address;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com");
    let signer = read_keypair_file("/path/to/your/keypair.json")?;
    let owner = signer.pubkey();

    // --- 1. Fetch vault and reserve data ----------------------------------------------------

    let vault_pubkey = Pubkey::from_str("HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E")?;
    let reserve_pubkey = Pubkey::from_str("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59")?;

    let vault_data = rpc_client.get_account(&vault_pubkey)?;

    // Fetch the disinvest target reserve...
    let reserve_data = rpc_client.get_account(&reserve_pubkey)?;
    let reserve = ReserveInfo::from_account_data(reserve_pubkey, &reserve_data.data)?;

    // ...and every active reserve so we can read its lending market — the
    // on-chain refresh needs both the reserve and its lending market. The
    // disinvest target is among them, so reuse the ReserveInfo we already have.
    let vault_state = from_account_data::<VaultState>(&vault_data.data)?;
    let mut reserve_infos = Vec::new();
    for active_reserve in VaultInfo::active_reserve_addresses(vault_state) {
        if active_reserve == reserve.address {
            reserve_infos.push(ReserveInfo::from_account_data(
                reserve.address,
                &reserve_data.data,
            )?);
        } else {
            let data = rpc_client.get_account(&active_reserve)?;
            reserve_infos.push(ReserveInfo::from_account_data(active_reserve, &data.data)?);
        }
    }

    let vault = VaultInfo::from_account_data(vault_pubkey, &vault_data.data, &reserve_infos)?;

    // --- 2. Derive user token accounts ------------------------------------------------------

    let user_token_ata = get_associated_token_address(&owner, &vault.token_mint);
    let (shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_pubkey);
    let user_shares_ata = get_associated_token_address(&owner, &shares_mint);

    // --- 3. Build withdraw instruction ------------------------------------------------------

    let ix = helpers::withdraw::withdraw(
        &vault,
        owner,
        user_token_ata,
        user_shares_ata,
        &reserve,
        1_000_000, // shares to burn
    );

    // --- 4. Send transaction ----------------------------------------------------------------

    let message = solana_sdk::message::Message::new(&[ix], Some(&owner));
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = solana_sdk::transaction::Transaction::new(&[&signer], message, recent_blockhash);
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("Withdrawal successful! Signature: {signature}");

    Ok(())
}

//! Deposit tokens into a vault and receive shares.
//!
//! Demonstrates the simplest vault interaction — you supply underlying tokens
//! and receive vault shares that represent your proportional ownership.
//!
//! NOTE: depositing only mints the shares to the user's share ATA. If the
//! vault has a share farm (`VaultState::vault_farm` is set), the user must
//! also stake those shares into the farm to receive vault-level rewards — the
//! shares are not staked automatically. Staking is a separate Kamino Farms
//! instruction (appended after this deposit).
//!
//! ```text
//! cargo run --example deposit
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

    // --- 1. Fetch vault data ----------------------------------------------------------------

    let vault_pubkey = Pubkey::from_str("HDsayqAsDWy3QvANGqh2yNraqcD8Fnjgh73Mhb3WRS5E")?;

    let vault_data = rpc_client.get_account(&vault_pubkey)?;

    // Fetch every active reserve so we can read its lending market — the
    // on-chain refresh needs both the reserve and its lending market.
    let vault_state = from_account_data::<VaultState>(&vault_data.data)?;
    let mut reserve_infos = Vec::new();
    for reserve_pubkey in VaultInfo::active_reserve_addresses(vault_state) {
        let reserve_data = rpc_client.get_account(&reserve_pubkey)?;
        reserve_infos.push(ReserveInfo::from_account_data(
            reserve_pubkey,
            &reserve_data.data,
        )?);
    }

    let vault = VaultInfo::from_account_data(vault_pubkey, &vault_data.data, &reserve_infos)?;

    // --- 2. Derive user token accounts ------------------------------------------------------

    // User's ATA for the underlying token (e.g. USDC)
    let user_token_ata = get_associated_token_address(&owner, &vault.token_mint);

    // User's ATA for the vault shares mint
    let (shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_pubkey);
    let user_shares_ata = get_associated_token_address(&owner, &shares_mint);

    // --- 3. Build deposit instruction -------------------------------------------------------

    let ix = helpers::deposit::deposit(
        &vault,
        owner,
        user_token_ata,  // source: user's token account
        user_shares_ata, // destination: user's shares account
        1_000_000,       // 1 USDC (6 decimals)
    );

    // --- 4. Send transaction ----------------------------------------------------------------

    let message = solana_sdk::message::Message::new(&[ix], Some(&owner));
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = solana_sdk::transaction::Transaction::new(&[&signer], message, recent_blockhash);
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("Deposit successful! Signature: {signature}");

    // To receive vault-level rewards, the freshly minted shares must be staked
    // into the vault's farm (`vault.vault_farm`) if it has one — see the
    // module-level note.

    Ok(())
}

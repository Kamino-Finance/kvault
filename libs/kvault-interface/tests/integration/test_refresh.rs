//! Regression test for the refresh remaining-accounts fix.
//!
//! The on-chain `cpi_refresh_reserves` builds `[reserve, lending_market]`
//! pairs for klend's `RefreshReservesBatch` and requires every reserve's
//! lending-market account to be present in the transaction. The high-level
//! helpers must therefore append the refresh list as two slot-ordered blocks:
//! writable reserves followed by readonly lending markets.
//!
//! Before the fix, `refresh_remaining_accounts` appended only the reserves, so
//! any helper-built transaction was missing the lending-market accounts. With a
//! single reserve the older tests masked this with a test-only
//! `append_lending_markets` shim; here we use two reserves across two distinct
//! lending markets and execute the deposit on-chain WITHOUT any shim, so the
//! library is solely responsible for supplying both lending markets.

use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::setup;

#[test]
fn test_refresh_appends_all_lending_markets_across_two_markets() {
    let mut env = setup::setup_full_env();
    let (market2, reserve2) = setup::add_second_market_and_reserve(&mut env);

    let vault_info = setup::build_vault_info(&env);

    // The vault now spans two reserves in two distinct lending markets.
    assert_eq!(
        vault_info.reserves().len(),
        2,
        "vault should have two active reserves"
    );
    let markets: Vec<_> = vault_info
        .reserves()
        .iter()
        .map(|r| r.lending_market)
        .collect();
    assert!(
        markets.contains(&env.lending_market.pubkey()),
        "first lending market must be present"
    );
    assert!(
        markets.contains(&market2.pubkey()),
        "second lending market must be present"
    );
    assert_ne!(
        markets[0], markets[1],
        "the two reserves must live in distinct lending markets"
    );
    assert!(
        vault_info
            .reserves()
            .iter()
            .any(|r| r.reserve == reserve2.pubkey()),
        "the second reserve must be an active allocation"
    );

    // Build a deposit via the high-level helper — no append_lending_markets shim.
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let deposit_amount = 1_000_000u64;
    let user_token_ata = setup::create_token_account(
        &mut env.svm,
        &env.admin,
        &env.liquidity_mint,
        &user.pubkey(),
    );
    setup::mint_to(
        &mut env.svm,
        &env.admin,
        &env.liquidity_mint,
        &user_token_ata,
        deposit_amount,
    );
    let (shares_mint, _) = kvault_interface::pda::shares_mint(
        &kvault_interface::KVAULT_PROGRAM_ID,
        &vault_info.address,
    );
    let user_shares_ata =
        setup::create_token_account(&mut env.svm, &env.admin, &shares_mint, &user.pubkey());

    let ix = kvault_interface::helpers::deposit(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        deposit_amount,
    );

    // Layout check: the trailing remaining accounts are two slot-ordered blocks
    // — writable reserves first, then readonly lending markets, in the same
    // order as `vault_info.reserves()`.
    let n = vault_info.reserves().len();
    let tail = &ix.accounts[ix.accounts.len() - 2 * n..];
    for (i, r) in vault_info.reserves().iter().enumerate() {
        assert_eq!(tail[i].pubkey, r.reserve, "reserve block, slot {i}");
        assert!(
            tail[i].is_writable && !tail[i].is_signer,
            "reserve meta must be writable, slot {i}"
        );
        assert_eq!(
            tail[n + i].pubkey,
            r.lending_market,
            "lending-market block, slot {i}"
        );
        assert!(
            !tail[n + i].is_writable && !tail[n + i].is_signer,
            "lending-market meta must be readonly, slot {i}"
        );
    }

    // Execute on-chain. With two markets this only succeeds if BOTH lending
    // markets are present — the regression: it fails against the pre-fix
    // refresh_remaining_accounts (which omitted the lending markets).
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm
        .send_transaction(tx)
        .expect("deposit must succeed with both lending markets present");

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert!(shares_bal > 0, "user should have received shares");
}

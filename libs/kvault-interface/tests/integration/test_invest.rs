use kvault_interface::pda;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::setup;

#[test]
fn test_invest() {
    let mut env = setup::setup_full_env();

    // First deposit some tokens
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

    let vault_info = setup::build_vault_info(&env);
    let (shares_mint, _) =
        pda::shares_mint(&kvault_interface::KVAULT_PROGRAM_ID, &vault_info.address);
    let user_shares_ata =
        setup::create_token_account(&mut env.svm, &env.admin, &shares_mint, &user.pubkey());

    let deposit_ix = kvault_interface::helpers::deposit(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        deposit_amount,
    );

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    // Read token_vault balance before invest
    let (token_vault, _) =
        pda::token_vault(&kvault_interface::KVAULT_PROGRAM_ID, &vault_info.address);
    let token_vault_before = setup::token_balance(&mut env.svm, &token_vault);
    assert!(
        token_vault_before > 0,
        "Token vault should have tokens after deposit"
    );

    // Create payer token account for invest
    let payer = Keypair::new();
    env.svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_token_ata =
        setup::create_token_account(&mut env.svm, &payer, &env.liquidity_mint, &payer.pubkey());

    let vault_info = setup::build_vault_info(&env);
    let reserve_info = setup::build_reserve_info(&env);

    let invest_ix = kvault_interface::helpers::invest(
        &vault_info,
        payer.pubkey(),
        payer_token_ata,
        &reserve_info,
        false,
    );

    let tx = Transaction::new_signed_with_payer(
        &[invest_ix],
        Some(&payer.pubkey()),
        &[&payer],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    // Verify: token vault decreased
    let token_vault_after = setup::token_balance(&mut env.svm, &token_vault);
    assert!(
        token_vault_after < token_vault_before,
        "Token vault should decrease after invest"
    );

    // Verify: ctoken vault has tokens
    let (ctoken_vault, _) = pda::ctoken_vault(
        &kvault_interface::KVAULT_PROGRAM_ID,
        &vault_info.address,
        &env.reserve.pubkey(),
    );
    let ctoken_balance = setup::token_balance(&mut env.svm, &ctoken_vault);
    assert!(
        ctoken_balance > 0,
        "CToken vault should have tokens after invest"
    );
}

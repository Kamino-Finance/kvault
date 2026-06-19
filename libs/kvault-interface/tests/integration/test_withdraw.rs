use kvault_interface::pda;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::setup;

/// Helper: deposit tokens and return (user, user_token_ata, user_shares_ata)
fn deposit_tokens(
    env: &mut setup::TestEnv,
    amount: u64,
) -> (
    Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
) {
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

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
        amount,
    );

    let vault_info = setup::build_vault_info(env);
    let (shares_mint, _) =
        pda::shares_mint(&kvault_interface::KVAULT_PROGRAM_ID, &vault_info.address);
    let user_shares_ata =
        setup::create_token_account(&mut env.svm, &env.admin, &shares_mint, &user.pubkey());

    let ix = kvault_interface::helpers::deposit(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        amount,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    (user, user_token_ata, user_shares_ata)
}

/// Helper: invest vault tokens into klend reserve
fn invest(env: &mut setup::TestEnv) {
    let payer = Keypair::new();
    env.svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_token_ata =
        setup::create_token_account(&mut env.svm, &payer, &env.liquidity_mint, &payer.pubkey());

    let vault_info = setup::build_vault_info(env);
    let reserve_info = setup::build_reserve_info(env);

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
}

#[test]
fn test_withdraw_from_available() {
    let mut env = setup::setup_full_env();

    let deposit_amount = 1_000_000u64;
    let (user, user_token_ata, user_shares_ata) = deposit_tokens(&mut env, deposit_amount);

    let shares_balance = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert!(shares_balance > 0);

    let vault_info = setup::build_vault_info(&env);

    let ix = kvault_interface::helpers::withdraw_from_available(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        shares_balance,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    let token_bal = setup::token_balance(&mut env.svm, &user_token_ata);
    assert!(token_bal > 0, "User should have received tokens back");

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert_eq!(shares_bal, 0, "All shares should be burned");
}

#[test]
fn test_withdraw_after_invest() {
    let mut env = setup::setup_full_env();

    let deposit_amount = 1_000_000u64;
    let (user, user_token_ata, user_shares_ata) = deposit_tokens(&mut env, deposit_amount);

    let shares_balance = setup::token_balance(&mut env.svm, &user_shares_ata);

    invest(&mut env);

    setup::advance_clock_by_slots(&mut env.svm, 10);

    let vault_info = setup::build_vault_info(&env);
    let reserve_info = setup::build_reserve_info(&env);

    let withdraw_ix = kvault_interface::helpers::withdraw(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        &reserve_info,
        shares_balance,
    );

    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    let token_bal = setup::token_balance(&mut env.svm, &user_token_ata);
    assert!(token_bal > 0, "User should have received tokens");

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert_eq!(shares_bal, 0, "All shares should be burned");
}

#[test]
fn test_sell() {
    let mut env = setup::setup_full_env();

    let deposit_amount = 1_000_000u64;
    let (user, user_token_ata, user_shares_ata) = deposit_tokens(&mut env, deposit_amount);

    let shares_balance = setup::token_balance(&mut env.svm, &user_shares_ata);

    invest(&mut env);

    setup::advance_clock_by_slots(&mut env.svm, 10);

    let vault_info = setup::build_vault_info(&env);
    let reserve_info = setup::build_reserve_info(&env);

    let sell_ix = kvault_interface::helpers::sell(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        &reserve_info,
        shares_balance,
    );

    let tx = Transaction::new_signed_with_payer(
        &[sell_ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    let token_bal = setup::token_balance(&mut env.svm, &user_token_ata);
    assert!(token_bal > 0, "User should have received tokens via sell");

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert_eq!(shares_bal, 0);
}

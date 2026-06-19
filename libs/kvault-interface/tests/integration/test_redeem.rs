use kvault_interface::pda;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::setup;

#[test]
fn test_redeem_in_kind() {
    let mut env = setup::setup_full_env();

    // Deposit
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

    // Invest
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

    // Prepare for redeem: user needs a cToken account
    let reserve_info = setup::build_reserve_info(&env);
    let user_ctoken_ta = setup::create_token_account(
        &mut env.svm,
        &env.admin,
        &reserve_info.collateral_mint,
        &user.pubkey(),
    );

    let shares_balance = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert!(shares_balance > 0);

    // Redeem in kind
    let vault_info = setup::build_vault_info(&env);

    let redeem_ix = kvault_interface::helpers::redeem_in_kind(
        &vault_info,
        user.pubkey(),
        &reserve_info,
        user_ctoken_ta,
        user_shares_ata,
        shares_balance,
    );

    let tx = Transaction::new_signed_with_payer(
        &[redeem_ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    // Verify: user received cTokens
    let ctoken_bal = setup::token_balance(&mut env.svm, &user_ctoken_ta);
    assert!(ctoken_bal > 0, "User should have received cTokens");

    // Verify: shares burned
    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert_eq!(shares_bal, 0, "All shares should be burned");
}

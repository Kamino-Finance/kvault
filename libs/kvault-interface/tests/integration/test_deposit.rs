use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::setup;

#[test]
fn test_deposit() {
    let mut env = setup::setup_full_env();

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

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    let token_bal = setup::token_balance(&mut env.svm, &user_token_ata);
    assert_eq!(token_bal, 0);

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert!(shares_bal > 0, "User should have received shares");
}

#[test]
fn test_buy() {
    let mut env = setup::setup_full_env();

    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

    let deposit_amount = 500_000u64;

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
    let (shares_mint, _) = kvault_interface::pda::shares_mint(
        &kvault_interface::KVAULT_PROGRAM_ID,
        &vault_info.address,
    );
    let user_shares_ata =
        setup::create_token_account(&mut env.svm, &env.admin, &shares_mint, &user.pubkey());

    let ix = kvault_interface::helpers::buy(
        &vault_info,
        user.pubkey(),
        user_token_ata,
        user_shares_ata,
        deposit_amount,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).unwrap();

    let token_bal = setup::token_balance(&mut env.svm, &user_token_ata);
    assert_eq!(token_bal, 0);

    let shares_bal = setup::token_balance(&mut env.svm, &user_shares_ata);
    assert!(shares_bal > 0, "User should have received shares via buy");
}

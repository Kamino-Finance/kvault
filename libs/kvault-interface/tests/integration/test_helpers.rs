use kvault_interface::discriminators::{
    compute_discriminator, identify_instruction, KvaultInstruction, BUY, BUY_WITH_MIN_SHARES_OUT,
    DEPOSIT, DEPOSIT_WITH_MIN_SHARES_OUT, INVEST, INVEST_WITH_MAX_AMOUNT, REDEEM_IN_KIND, SELL,
    WITHDRAW, WITHDRAW_FROM_AVAILABLE,
};
use kvault_interface::errors::KvaultError;
use kvault_interface::pda;
use kvault_interface::KVAULT_PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;

// ---------------------------------------------------------------------------
// Discriminator tests
// ---------------------------------------------------------------------------

#[test]
fn discriminators_match_computed() {
    assert_eq!(compute_discriminator("deposit"), DEPOSIT);
    assert_eq!(
        compute_discriminator("deposit_with_min_shares_out"),
        DEPOSIT_WITH_MIN_SHARES_OUT
    );
    assert_eq!(compute_discriminator("buy"), BUY);
    assert_eq!(
        compute_discriminator("buy_with_min_shares_out"),
        BUY_WITH_MIN_SHARES_OUT
    );
    assert_eq!(compute_discriminator("withdraw"), WITHDRAW);
    assert_eq!(compute_discriminator("sell"), SELL);
    assert_eq!(
        compute_discriminator("withdraw_from_available"),
        WITHDRAW_FROM_AVAILABLE
    );
    assert_eq!(compute_discriminator("invest"), INVEST);
    assert_eq!(
        compute_discriminator("invest_with_max_amount"),
        INVEST_WITH_MAX_AMOUNT
    );
    assert_eq!(compute_discriminator("redeem_in_kind"), REDEEM_IN_KIND);
}

#[test]
fn identify_instruction_roundtrips() {
    let cases = vec![
        (DEPOSIT, KvaultInstruction::Deposit),
        (
            DEPOSIT_WITH_MIN_SHARES_OUT,
            KvaultInstruction::DepositWithMinSharesOut,
        ),
        (BUY, KvaultInstruction::Buy),
        (
            BUY_WITH_MIN_SHARES_OUT,
            KvaultInstruction::BuyWithMinSharesOut,
        ),
        (WITHDRAW, KvaultInstruction::Withdraw),
        (SELL, KvaultInstruction::Sell),
        (
            WITHDRAW_FROM_AVAILABLE,
            KvaultInstruction::WithdrawFromAvailable,
        ),
        (INVEST, KvaultInstruction::Invest),
        (
            INVEST_WITH_MAX_AMOUNT,
            KvaultInstruction::InvestWithMaxAmount,
        ),
        (REDEEM_IN_KIND, KvaultInstruction::RedeemInKind),
    ];

    for (disc, expected) in cases {
        let mut data = disc.to_vec();
        data.extend_from_slice(&[0u8; 8]); // some trailing data
        assert_eq!(identify_instruction(&data), Some(expected));
    }
}

#[test]
fn identify_instruction_unknown() {
    assert_eq!(identify_instruction(&[0u8; 8]), None);
    assert_eq!(identify_instruction(&[0u8; 4]), None); // too short
    assert_eq!(identify_instruction(&[]), None);
}

// ---------------------------------------------------------------------------
// PDA tests
// ---------------------------------------------------------------------------

#[test]
fn pda_functions_are_deterministic() {
    let vault_state = Pubkey::new_unique();
    let reserve = Pubkey::new_unique();

    let (auth1, bump1) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, &vault_state);
    let (auth2, bump2) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, &vault_state);
    assert_eq!(auth1, auth2);
    assert_eq!(bump1, bump2);

    let (tv1, _) = pda::token_vault(&KVAULT_PROGRAM_ID, &vault_state);
    let (tv2, _) = pda::token_vault(&KVAULT_PROGRAM_ID, &vault_state);
    assert_eq!(tv1, tv2);

    let (sm1, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_state);
    let (sm2, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &vault_state);
    assert_eq!(sm1, sm2);

    let (cv1, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault_state, &reserve);
    let (cv2, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault_state, &reserve);
    assert_eq!(cv1, cv2);

    let (gc1, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let (gc2, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    assert_eq!(gc1, gc2);

    let (wr1, _) = pda::whitelisted_reserve(&KVAULT_PROGRAM_ID, &reserve);
    let (wr2, _) = pda::whitelisted_reserve(&KVAULT_PROGRAM_ID, &reserve);
    assert_eq!(wr1, wr2);

    let (ea1, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    let (ea2, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    assert_eq!(ea1, ea2);
}

#[test]
fn pda_different_inputs_different_outputs() {
    let vault1 = Pubkey::new_unique();
    let vault2 = Pubkey::new_unique();

    let (auth1, _) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, &vault1);
    let (auth2, _) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, &vault2);
    assert_ne!(auth1, auth2);
}

// ---------------------------------------------------------------------------
// Error tests
// ---------------------------------------------------------------------------

#[test]
fn error_code_roundtrip() {
    let errors = vec![
        KvaultError::DepositAmountsZero,
        KvaultError::MathOverflow,
        KvaultError::WithdrawAmountBelowMinimum,
        KvaultError::ReserveAlreadyExists,
        KvaultError::AdminAuthorityIncorrect,
        KvaultError::AUMDecreasedMoreThanExpected,
        KvaultError::MaxInvestAmountMustBeGreaterThanZero,
    ];

    for err in errors {
        let code = err.error_code();
        let roundtripped = KvaultError::from_error_code(code);
        assert_eq!(roundtripped, Some(err), "roundtrip failed for {err:?}");
    }
}

#[test]
fn error_code_unknown() {
    assert_eq!(KvaultError::from_error_code(5000), None);
    assert_eq!(KvaultError::from_error_code(9999), None);
    assert_eq!(KvaultError::from_error_code(0), None);
}

#[test]
fn error_display_formatting() {
    let err = KvaultError::DepositAmountsZero;
    let s = format!("{err}");
    assert_eq!(s, "Cannot deposit zero tokens");
    assert_eq!(err.error_code(), 7000);
}

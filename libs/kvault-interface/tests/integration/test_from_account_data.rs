use kvault_interface::{
    helpers::{ReserveInfo, VaultInfo},
    pda,
    state::{from_account_data, AccountDataError, GlobalConfig, VaultState},
    KVAULT_PROGRAM_ID,
};
use solana_sdk::signer::Signer;
use spl_discriminator::SplDiscriminate;

use super::setup;

#[test]
fn test_vault_state_from_account_data() {
    let env = setup::setup_full_env();

    let account = env.svm.get_account(&env.vault_state.pubkey()).unwrap();
    let vault = from_account_data::<VaultState>(&account.data).unwrap();

    // Admin should match
    assert_eq!(
        vault.vault_admin_authority.as_ref(),
        env.admin.pubkey().as_ref(),
        "vault admin should match"
    );

    // Token mint should match
    assert_eq!(
        vault.token_mint.as_ref(),
        env.liquidity_mint.as_ref(),
        "token mint should match"
    );

    // Shares mint should be the PDA
    let (expected_shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, &env.vault_state.pubkey());
    assert_eq!(
        vault.shares_mint.as_ref(),
        expected_shares_mint.as_ref(),
        "shares mint should match PDA"
    );
}

#[test]
fn test_global_config_from_account_data() {
    let env = setup::setup_full_env();

    let (gc_key, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let account = env.svm.get_account(&gc_key).unwrap();
    let config = from_account_data::<GlobalConfig>(&account.data).unwrap();

    assert_eq!(
        config.global_admin.as_ref(),
        env.admin.pubkey().as_ref(),
        "global_admin should match"
    );
}

#[test]
fn test_vault_info_from_account_data() {
    let env = setup::setup_full_env();

    let account = env.svm.get_account(&env.vault_state.pubkey()).unwrap();

    // Build the ReserveInfo list for the vault's active reserves.
    let vault_state = from_account_data::<VaultState>(&account.data).unwrap();
    let reserve_infos: Vec<ReserveInfo> = VaultInfo::active_reserve_addresses(vault_state)
        .into_iter()
        .map(|reserve_addr| {
            let data = env.svm.get_account(&reserve_addr).unwrap().data;
            ReserveInfo::from_account_data(reserve_addr, &data).unwrap()
        })
        .collect();

    let vault_info =
        VaultInfo::from_account_data(env.vault_state.pubkey(), &account.data, &reserve_infos)
            .unwrap();

    assert_eq!(vault_info.address, env.vault_state.pubkey());

    assert_eq!(vault_info.token_mint.as_ref(), env.liquidity_mint.as_ref());

    // Should have at least one reserve (we added one in setup)
    assert!(
        !vault_info.reserves().is_empty(),
        "VaultInfo should have at least one reserve"
    );

    // The reserve's lending market must match the env lending market.
    assert_eq!(
        vault_info.reserves()[0].lending_market,
        env.lending_market.pubkey()
    );
}

#[test]
fn test_reserve_info_from_account_data() {
    let env = setup::setup_full_env();

    let account = env.svm.get_account(&env.reserve.pubkey()).unwrap();
    let reserve_info = ReserveInfo::from_account_data(env.reserve.pubkey(), &account.data).unwrap();

    assert_eq!(reserve_info.address, env.reserve.pubkey());

    assert_eq!(
        reserve_info.lending_market.as_ref(),
        env.lending_market.pubkey().as_ref(),
        "lending_market should match"
    );
}

#[test]
fn test_wrong_discriminator() {
    let mut data = vec![0u8; 8 + core::mem::size_of::<VaultState>()];
    data[..8].copy_from_slice(&[0xFF; 8]);

    let result = from_account_data::<VaultState>(&data);
    assert!(matches!(
        result,
        Err(AccountDataError::InvalidDiscriminator { .. })
    ));
}

#[test]
fn test_truncated_data() {
    let disc = VaultState::SPL_DISCRIMINATOR_SLICE;
    let mut data = vec![0u8; 16]; // way too short
    data[..8].copy_from_slice(disc);

    let result = from_account_data::<VaultState>(&data);
    assert!(matches!(result, Err(AccountDataError::DataTooShort { .. })));
}

#[test]
fn test_vault_info_missing_reserve() {
    // Building a VaultInfo without supplying the active reserve's ReserveInfo
    // must fail with MissingReserve, naming the offending reserve.
    let env = setup::setup_full_env();

    let account = env.svm.get_account(&env.vault_state.pubkey()).unwrap();
    let result = VaultInfo::from_account_data(env.vault_state.pubkey(), &account.data, &[]);

    assert_eq!(
        result.err(),
        Some(kvault_interface::VaultInfoError::MissingReserve(
            env.reserve.pubkey()
        ))
    );
}

#[test]
fn test_reserve_info_try_from() {
    let env = setup::setup_full_env();

    let account = env.svm.get_account(&env.reserve.pubkey()).unwrap();
    let reserve_info: ReserveInfo = (env.reserve.pubkey(), account.data.as_slice())
        .try_into()
        .unwrap();

    assert_eq!(reserve_info.address, env.reserve.pubkey());
}

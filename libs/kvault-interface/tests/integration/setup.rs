use std::path::PathBuf;

use klend_interface::{
    discriminators::compute_discriminator as klend_compute_discriminator, pda as klend_pda,
    FARMS_PROGRAM_ID, KLEND_PROGRAM_ID, SYSTEM_PROGRAM_ID, SYSVAR_RENT_ID, TOKEN_PROGRAM_ID,
};
use kvault_interface::{
    discriminators::compute_discriminator, helpers::ReserveInfo, helpers::VaultInfo, pda,
    KVAULT_PROGRAM_ID,
};
use litesvm::LiteSVM;
pub use solana_sdk::instruction::Instruction;
use solana_sdk::{
    account::Account, clock::Clock, instruction::AccountMeta, program_pack::Pack, pubkey::Pubkey,
    signature::Keypair, signer::Signer, transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;
use spl_token::instruction as spl_ix;

use super::pyth;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// LendingMarket account size: 8 (disc) + 4656 (data).
const LENDING_MARKET_SIZE: usize = 8 + 4656;

/// Reserve account size: 8 (disc) + 8616 (data).
const RESERVE_SIZE: usize = 8 + 8616;

/// klend GlobalConfig account size: 8 (disc) + 1024 (data).
const KLEND_GLOBAL_CONFIG_SIZE: usize = 8 + 1024;

/// kvault GlobalConfig account size: 8 (disc) + 1024 (data).
const KVAULT_GLOBAL_CONFIG_SIZE: usize = 8 + 1024;

/// VaultState account size: 8 (disc) + 62544 (data).
const VAULT_STATE_SIZE: usize = 8 + 62544;

/// Default min initial deposit for klend reserves.
const MIN_INITIAL_DEPOSIT: u64 = 100_000;

/// Initial deposit into the vault during init_vault (mirrors the program's INITIAL_DEPOSIT_AMOUNT).
const INITIAL_DEPOSIT_AMOUNT: u64 = 1_000;

// ---------------------------------------------------------------------------
// TestEnv
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct TestEnv {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub lending_market: Keypair,
    pub reserve: Keypair,
    pub liquidity_mint: Pubkey,
    pub pyth_oracle: Pubkey,
    pub vault_state: Keypair,
}

// ---------------------------------------------------------------------------
// Full environment setup
// ---------------------------------------------------------------------------

pub fn setup_full_env() -> TestEnv {
    let mut svm = LiteSVM::new()
        .with_transaction_history(0)
        .with_lamports((100_000.0_f64 * 1_000_000_000.0) as u64);
    svm.airdrop(&Pubkey::new_unique(), 100_000_000_000).unwrap();

    // Load the kvault program .so
    let kvault_so_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/kamino_vault.so");
    let kvault_bytes = std::fs::read(&kvault_so_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", kvault_so_path.display()));
    svm.add_program(KVAULT_PROGRAM_ID, &kvault_bytes).unwrap();

    // Load the klend program .so (pre-built fixture)
    let klend_so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../programs/kvault/tests/fixtures/kamino_lending.so");
    let klend_bytes = std::fs::read(&klend_so_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", klend_so_path.display()));
    svm.add_program(KLEND_PROGRAM_ID, &klend_bytes).unwrap();

    // Load klend .so as the farms program too (Anchor checks executability)
    svm.add_program(FARMS_PROGRAM_ID, &klend_bytes).unwrap();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap();

    // Inject klend GlobalConfig PDA
    inject_klend_global_config(&mut svm, &admin.pubkey());

    // Inject kvault GlobalConfig PDA
    inject_kvault_global_config(&mut svm, &admin.pubkey());

    // Create lending market
    let lending_market = Keypair::new();
    create_lending_market(&mut svm, &admin, &lending_market);

    // Create SPL token mint for liquidity (6 decimals)
    let liquidity_mint = create_mint(&mut svm, &admin, 6);

    // Create mock pyth oracle at price = 1.0
    let pyth_oracle = pyth::create_pyth_price_account(&mut svm, 1.0);

    // Init klend reserve
    let reserve = Keypair::new();
    init_reserve(&mut svm, &admin, &lending_market, &reserve, &liquidity_mint);

    // Configure the reserve to be usable
    configure_reserve(
        &mut svm,
        &admin,
        &lending_market.pubkey(),
        &reserve.pubkey(),
        &pyth_oracle,
    );

    // Init kvault vault
    let vault_state = Keypair::new();
    init_vault(&mut svm, &admin, &vault_state, &liquidity_mint);

    // Update reserve allocation (weight=100, cap=unlimited)
    update_reserve_allocation(
        &mut svm,
        &admin,
        &vault_state.pubkey(),
        &reserve,
        &liquidity_mint,
    );

    TestEnv {
        svm,
        admin,
        lending_market,
        reserve,
        liquidity_mint,
        pyth_oracle,
        vault_state,
    }
}

// ---------------------------------------------------------------------------
// GlobalConfig injection
// ---------------------------------------------------------------------------

fn inject_klend_global_config(svm: &mut LiteSVM, admin: &Pubkey) {
    let (gc_key, _) = klend_pda::global_config(&KLEND_PROGRAM_ID);

    let disc = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"account:GlobalConfig");
        let hash = h.finalize();
        let mut d = [0u8; 8];
        d.copy_from_slice(&hash[..8]);
        d
    };

    let mut data = vec![0u8; KLEND_GLOBAL_CONFIG_SIZE];
    data[..8].copy_from_slice(&disc);
    // global_admin at offset 8
    data[8..40].copy_from_slice(admin.as_ref());
    // pending_admin at offset 40
    data[40..72].copy_from_slice(admin.as_ref());
    // fee_collector at offset 72
    data[72..104].copy_from_slice(admin.as_ref());

    let account = Account {
        lamports: u32::MAX as u64,
        data,
        owner: KLEND_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(gc_key, account).unwrap();
}

fn inject_kvault_global_config(svm: &mut LiteSVM, admin: &Pubkey) {
    let (gc_key, _) = pda::global_config(&KVAULT_PROGRAM_ID);

    // Discriminator: sha256("account:GlobalConfig")[..8]
    // Same discriminator hash as klend (both use "account:GlobalConfig")
    let disc = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"account:GlobalConfig");
        let hash = h.finalize();
        let mut d = [0u8; 8];
        d.copy_from_slice(&hash[..8]);
        d
    };

    let mut data = vec![0u8; KVAULT_GLOBAL_CONFIG_SIZE];
    data[..8].copy_from_slice(&disc);
    // global_admin at offset 8
    data[8..40].copy_from_slice(admin.as_ref());

    let account = Account {
        lamports: u32::MAX as u64,
        data,
        owner: KVAULT_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(gc_key, account).unwrap();
}

// ---------------------------------------------------------------------------
// klend admin instruction builders
// ---------------------------------------------------------------------------

fn build_init_lending_market_ix(owner: &Pubkey, lending_market: &Pubkey) -> Instruction {
    let disc = klend_compute_discriminator("init_lending_market");
    let quote_currency: [u8; 32] =
        *b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

    let mut data = disc.to_vec();
    data.extend_from_slice(&quote_currency);

    let (lma, _) = klend_pda::lending_market_authority(&KLEND_PROGRAM_ID, lending_market);

    Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*lending_market, false),
            AccountMeta::new_readonly(lma, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSVAR_RENT_ID, false),
        ],
        data,
    }
}

fn build_init_reserve_ix(
    signer: &Pubkey,
    lending_market: &Pubkey,
    reserve: &Pubkey,
    liquidity_mint: &Pubkey,
    initial_liq_source: &Pubkey,
) -> Instruction {
    let disc = klend_compute_discriminator("init_reserve");
    let data = disc.to_vec();

    let (lma, _) = klend_pda::lending_market_authority(&KLEND_PROGRAM_ID, lending_market);
    let (liq_supply, _) = klend_pda::reserve_liquidity_supply(&KLEND_PROGRAM_ID, reserve);
    let (fee_vault, _) = klend_pda::reserve_fee_receiver(&KLEND_PROGRAM_ID, reserve);
    let (coll_mint, _) = klend_pda::reserve_collateral_mint(&KLEND_PROGRAM_ID, reserve);
    let (coll_supply, _) = klend_pda::reserve_collateral_supply(&KLEND_PROGRAM_ID, reserve);

    Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(*lending_market, false),
            AccountMeta::new_readonly(lma, false),
            AccountMeta::new(*reserve, false),
            AccountMeta::new_readonly(*liquidity_mint, false),
            AccountMeta::new(liq_supply, false),
            AccountMeta::new(fee_vault, false),
            AccountMeta::new(coll_mint, false),
            AccountMeta::new(coll_supply, false),
            AccountMeta::new(*initial_liq_source, false),
            AccountMeta::new_readonly(SYSVAR_RENT_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Borsh index for `UpdateConfigMode` variants.
fn update_config_borsh_index(repr_value: u8) -> u8 {
    const REPR_VALUES: &[u8] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
        49, 50, 51, 52, 53, 54, 55,
    ];
    REPR_VALUES
        .iter()
        .position(|&v| v == repr_value)
        .unwrap_or_else(|| panic!("Unknown UpdateConfigMode repr value: {repr_value}")) as u8
}

fn build_update_reserve_config_ix(
    signer: &Pubkey,
    lending_market: &Pubkey,
    reserve: &Pubkey,
    mode: u8,
    value: Vec<u8>,
    skip_validation: bool,
) -> Instruction {
    let disc = klend_compute_discriminator("update_reserve_config");

    let mut data = disc.to_vec();
    data.push(update_config_borsh_index(mode));
    data.extend_from_slice(&(value.len() as u32).to_le_bytes());
    data.extend_from_slice(&value);
    data.push(skip_validation as u8);

    let (gc_key, _) = klend_pda::global_config(&KLEND_PROGRAM_ID);

    Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new_readonly(gc_key, false),
            AccountMeta::new_readonly(*lending_market, false),
            AccountMeta::new(*reserve, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// kvault admin instruction builders
// ---------------------------------------------------------------------------

fn build_init_vault_ix(
    admin: &Pubkey,
    vault_state: &Pubkey,
    token_mint: &Pubkey,
    admin_token_ata: &Pubkey,
) -> Instruction {
    let disc = compute_discriminator("init_vault");
    let data = disc.to_vec();

    let (base_vault_authority, _) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, vault_state);
    let (token_vault, _) = pda::token_vault(&KVAULT_PROGRAM_ID, vault_state);
    let (shares_mint, _) = pda::shares_mint(&KVAULT_PROGRAM_ID, vault_state);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(*vault_state, true),
            AccountMeta::new_readonly(base_vault_authority, false),
            AccountMeta::new(token_vault, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(shares_mint, false),
            AccountMeta::new(*admin_token_ata, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSVAR_RENT_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // shares_token_program
        ],
        data,
    }
}

fn build_update_reserve_allocation_ix(
    signer: &Pubkey,
    vault_state: &Pubkey,
    reserve: &Pubkey,
    reserve_collateral_mint: &Pubkey,
    weight: u64,
    cap: u64,
) -> Instruction {
    let disc = compute_discriminator("update_reserve_allocation");

    let mut data = disc.to_vec();
    // Args (borsh): target_allocation_weight: u64, allocation_cap: u64
    data.extend_from_slice(&weight.to_le_bytes());
    data.extend_from_slice(&cap.to_le_bytes());

    let (base_vault_authority, _) = pda::base_vault_authority(&KVAULT_PROGRAM_ID, vault_state);
    let (ctoken_vault, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, vault_state, reserve);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new_readonly(base_vault_authority, false),
            AccountMeta::new(*reserve_collateral_mint, false),
            AccountMeta::new_readonly(*reserve, false),
            AccountMeta::new(ctoken_vault, false),
            // reserve_whitelist_entry: optional → program_id placeholder
            AccountMeta::new_readonly(KVAULT_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // reserve_collateral_token_program
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSVAR_RENT_ID, false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Market & reserve creation (klend)
// ---------------------------------------------------------------------------

fn create_lending_market(svm: &mut LiteSVM, admin: &Keypair, market: &Keypair) {
    let create_ix = system_instruction::create_account(
        &admin.pubkey(),
        &market.pubkey(),
        svm.minimum_balance_for_rent_exemption(LENDING_MARKET_SIZE),
        LENDING_MARKET_SIZE as u64,
        &KLEND_PROGRAM_ID,
    );
    let init_ix = build_init_lending_market_ix(&admin.pubkey(), &market.pubkey());

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&admin.pubkey()),
        &[admin, market],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
}

fn init_reserve(
    svm: &mut LiteSVM,
    admin: &Keypair,
    market: &Keypair,
    reserve: &Keypair,
    liquidity_mint: &Pubkey,
) {
    let admin_ta = create_token_account(svm, admin, liquidity_mint, &admin.pubkey());
    mint_to(svm, admin, liquidity_mint, &admin_ta, MIN_INITIAL_DEPOSIT);

    let create_ix = system_instruction::create_account(
        &admin.pubkey(),
        &reserve.pubkey(),
        svm.minimum_balance_for_rent_exemption(RESERVE_SIZE),
        RESERVE_SIZE as u64,
        &KLEND_PROGRAM_ID,
    );
    let init_ix = build_init_reserve_ix(
        &admin.pubkey(),
        &market.pubkey(),
        &reserve.pubkey(),
        liquidity_mint,
        &admin_ta,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&admin.pubkey()),
        &[admin, reserve],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
}

fn configure_reserve(
    svm: &mut LiteSVM,
    admin: &Keypair,
    lending_market: &Pubkey,
    reserve: &Pubkey,
    pyth_oracle: &Pubkey,
) {
    let updates: Vec<(u8, Vec<u8>)> = vec![
        (21, pyth_oracle.to_bytes().to_vec()),
        (18, 1_000_000u64.to_le_bytes().to_vec()),
        (24, build_flat_borrow_rate_curve(100)),
        (1, vec![75u8]),
        (3, vec![80u8]),
        (33, 100u64.to_le_bytes().to_vec()),
        (39, vec![0u8]),
    ];

    for (mode, value) in updates {
        let ix = build_update_reserve_config_ix(
            &admin.pubkey(),
            lending_market,
            reserve,
            mode,
            value,
            true,
        );
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[admin],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx).unwrap();
    }

    let post_activation: Vec<(u8, Vec<u8>)> = vec![
        (9, u64::MAX.to_le_bytes().to_vec()),
        (10, u64::MAX.to_le_bytes().to_vec()),
        (45, u64::MAX.to_le_bytes().to_vec()),
    ];
    for (mode, value) in post_activation {
        let ix = build_update_reserve_config_ix(
            &admin.pubkey(),
            lending_market,
            reserve,
            mode,
            value,
            false,
        );
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[admin],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx).unwrap();
    }
}

fn build_flat_borrow_rate_curve(rate_bps: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(88);
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&rate_bps.to_le_bytes());
    for _ in 1..11 {
        buf.extend_from_slice(&10000u32.to_le_bytes());
        buf.extend_from_slice(&rate_bps.to_le_bytes());
    }
    buf
}

// ---------------------------------------------------------------------------
// Vault creation (kvault)
// ---------------------------------------------------------------------------

fn init_vault(svm: &mut LiteSVM, admin: &Keypair, vault_state: &Keypair, liquidity_mint: &Pubkey) {
    // Pre-create the vault_state account owned by kvault
    let create_ix = system_instruction::create_account(
        &admin.pubkey(),
        &vault_state.pubkey(),
        svm.minimum_balance_for_rent_exemption(VAULT_STATE_SIZE),
        VAULT_STATE_SIZE as u64,
        &KVAULT_PROGRAM_ID,
    );

    // Create admin's token account and mint initial tokens
    let admin_token_ata = create_token_account(svm, admin, liquidity_mint, &admin.pubkey());
    mint_to(
        svm,
        admin,
        liquidity_mint,
        &admin_token_ata,
        INITIAL_DEPOSIT_AMOUNT,
    );

    let init_ix = build_init_vault_ix(
        &admin.pubkey(),
        &vault_state.pubkey(),
        liquidity_mint,
        &admin_token_ata,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&admin.pubkey()),
        &[admin, vault_state],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("init_vault failed: {e:?}"));
}

fn update_reserve_allocation(
    svm: &mut LiteSVM,
    admin: &Keypair,
    vault_state: &Pubkey,
    reserve: &Keypair,
    _liquidity_mint: &Pubkey,
) {
    let (coll_mint, _) = klend_pda::reserve_collateral_mint(&KLEND_PROGRAM_ID, &reserve.pubkey());

    let ix = build_update_reserve_allocation_ix(
        &admin.pubkey(),
        vault_state,
        &reserve.pubkey(),
        &coll_mint,
        100,      // weight
        u64::MAX, // cap
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("update_reserve_allocation failed: {e:?}"));
}

/// Add a second lending market + reserve and register it as a second active
/// allocation on the vault, so the vault spans two reserves across two
/// distinct lending markets. Returns the new `(lending_market, reserve)`.
///
/// Used by the refresh regression test: with two markets the on-chain
/// `RefreshReservesBatch` CPI requires *both* lending-market accounts to be
/// present in the transaction, so a helper that omits them fails on-chain.
pub fn add_second_market_and_reserve(env: &mut TestEnv) -> (Keypair, Keypair) {
    let market = Keypair::new();
    create_lending_market(&mut env.svm, &env.admin, &market);

    let reserve = Keypair::new();
    init_reserve(
        &mut env.svm,
        &env.admin,
        &market,
        &reserve,
        &env.liquidity_mint,
    );
    configure_reserve(
        &mut env.svm,
        &env.admin,
        &market.pubkey(),
        &reserve.pubkey(),
        &env.pyth_oracle,
    );
    update_reserve_allocation(
        &mut env.svm,
        &env.admin,
        &env.vault_state.pubkey(),
        &reserve,
        &env.liquidity_mint,
    );

    (market, reserve)
}

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

pub fn create_mint(svm: &mut LiteSVM, payer: &Keypair, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_ix = spl_ix::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&payer.pubkey()),
        &[payer, &mint],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    mint.pubkey()
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let ta = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Account::LEN);

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &ta.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_ix = spl_ix::initialize_account(&spl_token::id(), &ta.pubkey(), mint, owner).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&payer.pubkey()),
        &[payer, &ta],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    ta.pubkey()
}

pub fn mint_to(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
) {
    let ix = spl_ix::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
}

pub fn advance_clock_by_slots(svm: &mut LiteSVM, slots: u64) {
    let mut clock: Clock = svm.get_sysvar();
    clock.slot += slots;
    clock.unix_timestamp += slots as i64;
    svm.set_sysvar(&clock);
}

pub fn token_balance(svm: &mut LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm.get_account(token_account).unwrap();
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Vault/Reserve info builders
// ---------------------------------------------------------------------------

pub fn build_vault_info(env: &TestEnv) -> VaultInfo {
    let data = env.svm.get_account(&env.vault_state.pubkey()).unwrap().data;
    let vault_state =
        kvault_interface::from_account_data::<kvault_interface::state::VaultState>(&data).unwrap();

    // Fetch each active reserve so VaultInfo can carry its lending market.
    let reserve_infos: Vec<ReserveInfo> = VaultInfo::active_reserve_addresses(vault_state)
        .into_iter()
        .map(|reserve_addr| {
            let reserve_data = env.svm.get_account(&reserve_addr).unwrap().data;
            ReserveInfo::from_account_data(reserve_addr, &reserve_data).unwrap()
        })
        .collect();

    VaultInfo::from_account_data(env.vault_state.pubkey(), &data, &reserve_infos).unwrap()
}

pub fn build_reserve_info(env: &TestEnv) -> ReserveInfo {
    let data = env.svm.get_account(&env.reserve.pubkey()).unwrap().data;
    ReserveInfo::from_account_data(env.reserve.pubkey(), &data).unwrap()
}

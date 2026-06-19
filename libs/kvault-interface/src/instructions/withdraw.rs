use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{discriminators, pda, util::*, KVAULT_PROGRAM_ID};

// ---------------------------------------------------------------------------
// withdraw / sell — full withdraw (from available + from invested)
// ---------------------------------------------------------------------------

/// Account addresses required by the `withdraw` and `sell` instructions.
///
/// Combines accounts for both the "withdraw from available" and
/// "withdraw from invested" phases of a full withdrawal.
pub struct WithdrawAccounts {
    // WithdrawFromAvailable
    pub user: Pubkey,
    pub vault_state: Pubkey,
    pub global_config: Pubkey,
    pub token_vault: Pubkey,
    pub base_vault_authority: Pubkey,
    pub user_token_ata: Pubkey,
    pub token_mint: Pubkey,
    pub user_shares_ata: Pubkey,
    pub shares_mint: Pubkey,
    pub token_program: Pubkey,
    pub shares_token_program: Pubkey,
    pub klend_program: Pubkey,
    // WithdrawFromInvested
    pub invested_vault_state: Pubkey,
    pub reserve: Pubkey,
    pub ctoken_vault: Pubkey,
    pub lending_market: Pubkey,
    pub lending_market_authority: Pubkey,
    pub reserve_liquidity_supply: Pubkey,
    pub reserve_collateral_mint: Pubkey,
    pub reserve_collateral_token_program: Pubkey,
    pub instruction_sysvar_account: Pubkey,
}

fn build_withdraw_accounts(accounts: &WithdrawAccounts) -> Vec<AccountMeta> {
    let (evt_auth, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    vec![
        // WithdrawFromAvailable
        signer(accounts.user),
        writable(accounts.vault_state),
        readonly(accounts.global_config),
        writable(accounts.token_vault),
        readonly(accounts.base_vault_authority),
        writable(accounts.user_token_ata),
        writable(accounts.token_mint),
        writable(accounts.user_shares_ata),
        writable(accounts.shares_mint),
        readonly(accounts.token_program),
        readonly(accounts.shares_token_program),
        readonly(accounts.klend_program),
        // event_cpi (on WithdrawFromAvailable)
        readonly(evt_auth),
        readonly(KVAULT_PROGRAM_ID),
        // WithdrawFromInvested
        writable(accounts.invested_vault_state),
        writable(accounts.reserve),
        writable(accounts.ctoken_vault),
        readonly(accounts.lending_market),
        readonly(accounts.lending_market_authority),
        writable(accounts.reserve_liquidity_supply),
        writable(accounts.reserve_collateral_mint),
        readonly(accounts.reserve_collateral_token_program),
        readonly(accounts.instruction_sysvar_account),
        // event_cpi (on Withdraw)
        readonly(evt_auth),
        readonly(KVAULT_PROGRAM_ID),
    ]
}

/// Build a raw `withdraw` instruction.
///
/// Burns `shares_amount` vault shares and returns the underlying tokens.
/// If available liquidity is insufficient, the program automatically
/// disinvests from the specified reserve. Pass `u64::MAX` to burn the caller's
/// entire share balance — the program clamps `shares_amount` to the balance
/// actually held. `remaining_accounts` must contain the refresh list: writable
/// reserve metas followed by readonly lending-market metas, in allocation-slot
/// order.
///
/// # Farms
///
/// If the vault has a share farm (`VaultState::vault_farm` is set), the
/// caller's shares are staked in the farm and must be unstaked from it (a
/// separate Kamino Farms instruction, prepended to the transaction) before
/// they can be burned here.
pub fn withdraw(
    accounts: WithdrawAccounts,
    shares_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        shares_amount: u64,
    }

    let args = Args { shares_amount };
    let mut data = discriminators::WITHDRAW.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_withdraw_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

/// Build a raw `sell` instruction.
///
/// Same accounts and semantics as [`withdraw`], but uses the `sell`
/// discriminator. The amount is interpreted as a share amount; `u64::MAX`
/// burns the caller's entire balance, and the farm-unstake requirement
/// described on [`withdraw`] applies here too.
pub fn sell(
    accounts: WithdrawAccounts,
    shares_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        shares_amount: u64,
    }

    let args = Args { shares_amount };
    let mut data = discriminators::SELL.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_withdraw_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

// ---------------------------------------------------------------------------
// withdraw_from_available
// ---------------------------------------------------------------------------

/// Account addresses required by the `withdraw_from_available` instruction.
///
/// Only interacts with the vault's available (uninvested) liquidity — no
/// reserve disinvestment occurs.
pub struct WithdrawFromAvailableAccounts {
    pub user: Pubkey,
    pub vault_state: Pubkey,
    pub global_config: Pubkey,
    pub token_vault: Pubkey,
    pub base_vault_authority: Pubkey,
    pub user_token_ata: Pubkey,
    pub token_mint: Pubkey,
    pub user_shares_ata: Pubkey,
    pub shares_mint: Pubkey,
    pub token_program: Pubkey,
    pub shares_token_program: Pubkey,
    pub klend_program: Pubkey,
}

/// Build a raw `withdraw_from_available` instruction.
///
/// Burns `shares_amount` vault shares and returns tokens exclusively from
/// the vault's available balance. Fails if available liquidity is insufficient.
/// `remaining_accounts` must contain the refresh list: writable reserve metas
/// followed by readonly lending-market metas, in allocation-slot order.
pub fn withdraw_from_available(
    accounts: WithdrawFromAvailableAccounts,
    shares_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        shares_amount: u64,
    }

    let args = Args { shares_amount };
    let mut data = discriminators::WITHDRAW_FROM_AVAILABLE.to_vec();
    args.serialize(&mut data).unwrap();

    let (evt_auth, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    let mut account_metas = vec![
        signer(accounts.user),
        writable(accounts.vault_state),
        readonly(accounts.global_config),
        writable(accounts.token_vault),
        readonly(accounts.base_vault_authority),
        writable(accounts.user_token_ata),
        writable(accounts.token_mint),
        writable(accounts.user_shares_ata),
        writable(accounts.shares_mint),
        readonly(accounts.token_program),
        readonly(accounts.shares_token_program),
        readonly(accounts.klend_program),
        // event_cpi accounts
        readonly(evt_auth),
        readonly(KVAULT_PROGRAM_ID),
    ];
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

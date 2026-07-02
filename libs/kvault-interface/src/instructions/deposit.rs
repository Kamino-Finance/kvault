use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{discriminators, pda, util::*, KVAULT_PROGRAM_ID};

// ---------------------------------------------------------------------------
// Accounts shared by deposit and buy
// ---------------------------------------------------------------------------

/// Account addresses required by the `deposit` and `buy` instructions.
pub struct DepositAccounts {
    pub user: Pubkey,
    pub vault_state: Pubkey,
    pub token_vault: Pubkey,
    pub token_mint: Pubkey,
    pub base_vault_authority: Pubkey,
    pub shares_mint: Pubkey,
    pub user_token_ata: Pubkey,
    pub user_shares_ata: Pubkey,
    pub klend_program: Pubkey,
    pub token_program: Pubkey,
    pub shares_token_program: Pubkey,
}

fn build_deposit_accounts(accounts: &DepositAccounts) -> Vec<AccountMeta> {
    let (evt_auth, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    vec![
        signer_writable(accounts.user),
        writable(accounts.vault_state),
        writable(accounts.token_vault),
        readonly(accounts.token_mint),
        readonly(accounts.base_vault_authority),
        writable(accounts.shares_mint),
        writable(accounts.user_token_ata),
        writable(accounts.user_shares_ata),
        readonly(accounts.klend_program),
        readonly(accounts.token_program),
        readonly(accounts.shares_token_program),
        // event_cpi accounts
        readonly(evt_auth),
        readonly(KVAULT_PROGRAM_ID),
    ]
}

// ---------------------------------------------------------------------------
// deposit
// ---------------------------------------------------------------------------

/// Build a raw `deposit` instruction.
///
/// Transfers up to `max_amount` tokens from the user into the vault and
/// mints vault shares in return. `remaining_accounts` must contain the refresh
/// list: writable reserve metas followed by readonly lending-market metas, in
/// allocation-slot order.
pub fn deposit(
    accounts: DepositAccounts,
    max_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        max_amount: u64,
    }

    let args = Args { max_amount };
    let mut data = discriminators::DEPOSIT.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_deposit_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

// ---------------------------------------------------------------------------
// deposit_with_min_shares_out
// ---------------------------------------------------------------------------

/// Build a raw `deposit_with_min_shares_out` instruction.
///
/// Same accounts and semantics as [`deposit`], but includes `min_shares_out`
/// as a slippage guard. The instruction fails if fewer shares would be minted.
pub fn deposit_with_min_shares_out(
    accounts: DepositAccounts,
    max_amount: u64,
    min_shares_out: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        max_amount: u64,
        min_shares_out: u64,
    }

    let args = Args {
        max_amount,
        min_shares_out,
    };
    let mut data = discriminators::DEPOSIT_WITH_MIN_SHARES_OUT.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_deposit_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

// ---------------------------------------------------------------------------
// buy (same accounts and args as deposit, different discriminator)
// ---------------------------------------------------------------------------

/// Build a raw `buy` instruction.
///
/// Same accounts and semantics as [`deposit`], but uses the `buy`
/// discriminator. The amount is interpreted as the maximum tokens to spend.
pub fn buy(
    accounts: DepositAccounts,
    max_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        max_amount: u64,
    }

    let args = Args { max_amount };
    let mut data = discriminators::BUY.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_deposit_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

// ---------------------------------------------------------------------------
// buy_with_min_shares_out (same accounts and args as deposit_with_min_shares_out, different discriminator)
// ---------------------------------------------------------------------------

/// Build a raw `buy_with_min_shares_out` instruction.
///
/// Same accounts and semantics as [`deposit_with_min_shares_out`], but uses the `buy_with_min_shares_out`
/// discriminator.
pub fn buy_with_min_shares_out(
    accounts: DepositAccounts,
    max_amount: u64,
    min_shares_out: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        max_amount: u64,
        min_shares_out: u64,
    }

    let args = Args {
        max_amount,
        min_shares_out,
    };
    let mut data = discriminators::BUY_WITH_MIN_SHARES_OUT.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_deposit_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

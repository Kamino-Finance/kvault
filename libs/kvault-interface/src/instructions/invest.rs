use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{discriminators, util::*, KVAULT_PROGRAM_ID};

// ---------------------------------------------------------------------------
// invest
// ---------------------------------------------------------------------------

/// Account addresses required by the `invest` instruction.
pub struct InvestAccounts {
    pub payer: Pubkey,
    pub payer_token_account: Pubkey,
    pub vault_state: Pubkey,
    pub token_vault: Pubkey,
    pub token_mint: Pubkey,
    pub base_vault_authority: Pubkey,
    pub ctoken_vault: Pubkey,
    pub reserve: Pubkey,
    pub lending_market: Pubkey,
    pub lending_market_authority: Pubkey,
    pub reserve_liquidity_supply: Pubkey,
    pub reserve_collateral_mint: Pubkey,
    pub reserve_whitelist_entry: Option<Pubkey>,
    pub klend_program: Pubkey,
    pub reserve_collateral_token_program: Pubkey,
    pub token_program: Pubkey,
    pub instruction_sysvar_account: Pubkey,
}

fn build_invest_accounts(accounts: &InvestAccounts) -> Vec<AccountMeta> {
    vec![
        signer_writable(accounts.payer),
        writable(accounts.payer_token_account),
        writable(accounts.vault_state),
        writable(accounts.token_vault),
        writable(accounts.token_mint),
        writable(accounts.base_vault_authority),
        writable(accounts.ctoken_vault),
        writable(accounts.reserve),
        readonly(accounts.lending_market),
        readonly(accounts.lending_market_authority),
        writable(accounts.reserve_liquidity_supply),
        writable(accounts.reserve_collateral_mint),
        optional_account(&KVAULT_PROGRAM_ID, accounts.reserve_whitelist_entry, false),
        readonly(accounts.klend_program),
        readonly(accounts.reserve_collateral_token_program),
        readonly(accounts.token_program),
        readonly(accounts.instruction_sysvar_account),
    ]
}

/// Build a raw `invest` instruction.
///
/// Deploys available vault liquidity into a Klend reserve by depositing
/// tokens and receiving cTokens. Takes no explicit amount argument — the
/// program determines the amount based on the allocation strategy.
/// `remaining_accounts` must contain the refresh list: writable reserve metas
/// followed by readonly lending-market metas, in allocation-slot order.
pub fn invest(accounts: InvestAccounts, remaining_accounts: Vec<AccountMeta>) -> Instruction {
    // invest takes no args (empty borsh payload)
    let data = discriminators::INVEST.to_vec();

    let mut account_metas = build_invest_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

/// Build a raw `invest_with_max_amount` instruction.
///
/// Same accounts and refresh-list semantics as [`invest`], but caps the
/// liquidity moved toward the target allocation to `max_amount`.
pub fn invest_with_max_amount(
    accounts: InvestAccounts,
    max_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        max_amount: u64,
    }

    let args = Args { max_amount };
    let mut data = discriminators::INVEST_WITH_MAX_AMOUNT.to_vec();
    args.serialize(&mut data).unwrap();

    let mut account_metas = build_invest_accounts(&accounts);
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

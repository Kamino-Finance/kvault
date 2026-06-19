use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use super::common::{derive_vault_pdas, refresh_remaining_accounts};
use super::info::{ReserveInfo, VaultInfo};
use crate::instructions;
use crate::{pda, KLEND_PROGRAM_ID, KVAULT_PROGRAM_ID, SYSVAR_INSTRUCTIONS_ID, TOKEN_PROGRAM_ID};

/// Build an `invest` instruction.
///
/// Deploys available vault liquidity into a Klend `reserve` by depositing
/// tokens and receiving cTokens. The program determines the amount based on
/// the vault's allocation strategy. PDAs and remaining accounts are derived
/// automatically.
///
/// The `payer` must hold a few lamports of the vault's token in
/// `payer_token_account`: invest can incur a small rounding loss that is
/// charged to the payer, so an empty account will cause the instruction to
/// fail.
///
/// When `include_whitelist_entry` is `true`, the whitelisted-reserve PDA is
/// included; otherwise the optional account is set to `None` (program-ID
/// placeholder).
pub fn invest(
    vault: &VaultInfo,
    payer: Pubkey,
    payer_token_account: Pubkey,
    reserve: &ReserveInfo,
    include_whitelist_entry: bool,
) -> Instruction {
    let (accounts, remaining) = invest_accounts(
        vault,
        payer,
        payer_token_account,
        reserve,
        include_whitelist_entry,
    );

    instructions::invest::invest(accounts, remaining)
}

/// Build an `invest_with_max_amount` instruction.
///
/// Same PDA and account derivation behavior as [`invest`], but caps the
/// liquidity moved toward the target allocation to `max_amount`.
pub fn invest_with_max_amount(
    vault: &VaultInfo,
    payer: Pubkey,
    payer_token_account: Pubkey,
    reserve: &ReserveInfo,
    include_whitelist_entry: bool,
    max_amount: u64,
) -> Instruction {
    let (accounts, remaining) = invest_accounts(
        vault,
        payer,
        payer_token_account,
        reserve,
        include_whitelist_entry,
    );

    instructions::invest::invest_with_max_amount(accounts, max_amount, remaining)
}

fn invest_accounts(
    vault: &VaultInfo,
    payer: Pubkey,
    payer_token_account: Pubkey,
    reserve: &ReserveInfo,
    include_whitelist_entry: bool,
) -> (instructions::invest::InvestAccounts, Vec<AccountMeta>) {
    let pdas = derive_vault_pdas(&vault.address);
    let (ctoken_vault, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault.address, &reserve.address);
    let (lending_market_authority, _) =
        klend_interface::pda::lending_market_authority(&KLEND_PROGRAM_ID, &reserve.lending_market);
    let whitelist_entry = if include_whitelist_entry {
        Some(pda::whitelisted_reserve(&KVAULT_PROGRAM_ID, &reserve.address).0)
    } else {
        None
    };
    let remaining = refresh_remaining_accounts(vault);

    (
        instructions::invest::InvestAccounts {
            payer,
            payer_token_account,
            vault_state: vault.address,
            token_vault: pdas.token_vault,
            token_mint: vault.token_mint,
            base_vault_authority: pdas.base_vault_authority,
            ctoken_vault,
            reserve: reserve.address,
            lending_market: reserve.lending_market,
            lending_market_authority,
            reserve_liquidity_supply: reserve.liquidity_supply_vault,
            reserve_collateral_mint: reserve.collateral_mint,
            reserve_whitelist_entry: whitelist_entry,
            klend_program: KLEND_PROGRAM_ID,
            reserve_collateral_token_program: TOKEN_PROGRAM_ID,
            token_program: vault.token_program,
            instruction_sysvar_account: SYSVAR_INSTRUCTIONS_ID,
        },
        remaining,
    )
}

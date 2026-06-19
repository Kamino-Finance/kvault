use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use super::common::{derive_vault_pdas, refresh_remaining_accounts};
use super::info::{ReserveInfo, VaultInfo};
use crate::instructions;
use crate::{pda, KLEND_PROGRAM_ID, KVAULT_PROGRAM_ID, SYSVAR_INSTRUCTIONS_ID, TOKEN_PROGRAM_ID};

/// Build a `withdraw` instruction (from available + from invested via a single reserve).
///
/// Burns `shares_amount` vault shares and returns the underlying tokens.
/// If available liquidity is insufficient, the program automatically
/// disinvests from the specified `reserve`. PDAs and remaining accounts
/// are derived automatically.
pub fn withdraw(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    reserve: &ReserveInfo,
    shares_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let (global_config, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let (ctoken_vault, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault.address, &reserve.address);
    let (lending_market_authority, _) =
        klend_interface::pda::lending_market_authority(&KLEND_PROGRAM_ID, &reserve.lending_market);
    let remaining = refresh_remaining_accounts(vault);

    instructions::withdraw::withdraw(
        instructions::withdraw::WithdrawAccounts {
            user,
            vault_state: vault.address,
            global_config,
            token_vault: pdas.token_vault,
            base_vault_authority: pdas.base_vault_authority,
            user_token_ata,
            token_mint: vault.token_mint,
            user_shares_ata,
            shares_mint: pdas.shares_mint,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
            klend_program: KLEND_PROGRAM_ID,
            invested_vault_state: vault.address,
            reserve: reserve.address,
            ctoken_vault,
            lending_market: reserve.lending_market,
            lending_market_authority,
            reserve_liquidity_supply: reserve.liquidity_supply_vault,
            reserve_collateral_mint: reserve.collateral_mint,
            reserve_collateral_token_program: TOKEN_PROGRAM_ID,
            instruction_sysvar_account: SYSVAR_INSTRUCTIONS_ID,
        },
        shares_amount,
        remaining,
    )
}

/// Build a `sell` instruction.
///
/// Same accounts and semantics as [`withdraw`], but uses the `sell`
/// discriminator. The `shares_amount` is interpreted as shares to sell.
pub fn sell(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    reserve: &ReserveInfo,
    shares_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let (global_config, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let (ctoken_vault, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault.address, &reserve.address);
    let (lending_market_authority, _) =
        klend_interface::pda::lending_market_authority(&KLEND_PROGRAM_ID, &reserve.lending_market);
    let remaining = refresh_remaining_accounts(vault);

    instructions::withdraw::sell(
        instructions::withdraw::WithdrawAccounts {
            user,
            vault_state: vault.address,
            global_config,
            token_vault: pdas.token_vault,
            base_vault_authority: pdas.base_vault_authority,
            user_token_ata,
            token_mint: vault.token_mint,
            user_shares_ata,
            shares_mint: pdas.shares_mint,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
            klend_program: KLEND_PROGRAM_ID,
            invested_vault_state: vault.address,
            reserve: reserve.address,
            ctoken_vault,
            lending_market: reserve.lending_market,
            lending_market_authority,
            reserve_liquidity_supply: reserve.liquidity_supply_vault,
            reserve_collateral_mint: reserve.collateral_mint,
            reserve_collateral_token_program: TOKEN_PROGRAM_ID,
            instruction_sysvar_account: SYSVAR_INSTRUCTIONS_ID,
        },
        shares_amount,
        remaining,
    )
}

/// Build a `withdraw_from_available` instruction.
///
/// Burns `shares_amount` vault shares and returns tokens exclusively from
/// the vault's available (uninvested) balance. No reserve disinvestment
/// occurs — fails if available liquidity is insufficient. PDAs and
/// remaining accounts are derived automatically.
pub fn withdraw_from_available(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    shares_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let (global_config, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let remaining = refresh_remaining_accounts(vault);

    instructions::withdraw::withdraw_from_available(
        instructions::withdraw::WithdrawFromAvailableAccounts {
            user,
            vault_state: vault.address,
            global_config,
            token_vault: pdas.token_vault,
            base_vault_authority: pdas.base_vault_authority,
            user_token_ata,
            token_mint: vault.token_mint,
            user_shares_ata,
            shares_mint: pdas.shares_mint,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
            klend_program: KLEND_PROGRAM_ID,
        },
        shares_amount,
        remaining,
    )
}

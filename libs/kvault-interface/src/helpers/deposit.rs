use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use super::common::{derive_vault_pdas, refresh_remaining_accounts};
use super::info::VaultInfo;
use crate::instructions;
use crate::{KLEND_PROGRAM_ID, TOKEN_PROGRAM_ID};

/// Build a `deposit` instruction.
///
/// Transfers up to `max_amount` tokens from the user into the vault and
/// mints vault shares in return. PDAs and remaining accounts (reserve
/// refresh list) are derived automatically from [`VaultInfo`].
///
/// # Farms
///
/// If the vault has a share farm (`VaultState::vault_farm` is set), the shares
/// minted here are not automatically staked. To earn farm rewards the user
/// must stake them with a separate Kamino Farms instruction appended after
/// this one.
pub fn deposit(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    max_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let remaining = refresh_remaining_accounts(vault);

    instructions::deposit::deposit(
        instructions::deposit::DepositAccounts {
            user,
            vault_state: vault.address,
            token_vault: pdas.token_vault,
            token_mint: vault.token_mint,
            base_vault_authority: pdas.base_vault_authority,
            shares_mint: pdas.shares_mint,
            user_token_ata,
            user_shares_ata,
            klend_program: KLEND_PROGRAM_ID,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
        },
        max_amount,
        remaining,
    )
}

/// Build a `deposit_with_min_shares_out` instruction.
///
/// Same as [`deposit`], but includes `min_shares_out` as a slippage guard.
pub fn deposit_with_min_shares_out(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    max_amount: u64,
    min_shares_out: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let remaining = refresh_remaining_accounts(vault);

    instructions::deposit::deposit_with_min_shares_out(
        instructions::deposit::DepositAccounts {
            user,
            vault_state: vault.address,
            token_vault: pdas.token_vault,
            token_mint: vault.token_mint,
            base_vault_authority: pdas.base_vault_authority,
            shares_mint: pdas.shares_mint,
            user_token_ata,
            user_shares_ata,
            klend_program: KLEND_PROGRAM_ID,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
        },
        max_amount,
        min_shares_out,
        remaining,
    )
}

/// Build a `buy` instruction.
///
/// Same accounts and semantics as [`deposit`], but uses the `buy`
/// discriminator. The `max_amount` is interpreted as the maximum tokens
/// to spend. The farm note on [`deposit`] applies here too.
pub fn buy(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    max_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let remaining = refresh_remaining_accounts(vault);

    instructions::deposit::buy(
        instructions::deposit::DepositAccounts {
            user,
            vault_state: vault.address,
            token_vault: pdas.token_vault,
            token_mint: vault.token_mint,
            base_vault_authority: pdas.base_vault_authority,
            shares_mint: pdas.shares_mint,
            user_token_ata,
            user_shares_ata,
            klend_program: KLEND_PROGRAM_ID,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
        },
        max_amount,
        remaining,
    )
}

/// Build a `buy_with_min_shares_out` instruction.
///
/// Same as [`buy`], but includes `min_shares_out` as a slippage guard.
pub fn buy_with_min_shares_out(
    vault: &VaultInfo,
    user: Pubkey,
    user_token_ata: Pubkey,
    user_shares_ata: Pubkey,
    max_amount: u64,
    min_shares_out: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let remaining = refresh_remaining_accounts(vault);

    instructions::deposit::buy_with_min_shares_out(
        instructions::deposit::DepositAccounts {
            user,
            vault_state: vault.address,
            token_vault: pdas.token_vault,
            token_mint: vault.token_mint,
            base_vault_authority: pdas.base_vault_authority,
            shares_mint: pdas.shares_mint,
            user_token_ata,
            user_shares_ata,
            klend_program: KLEND_PROGRAM_ID,
            token_program: vault.token_program,
            shares_token_program: TOKEN_PROGRAM_ID,
        },
        max_amount,
        min_shares_out,
        remaining,
    )
}

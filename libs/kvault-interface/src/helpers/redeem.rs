use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use super::common::{derive_vault_pdas, refresh_remaining_accounts};
use super::info::{ReserveInfo, VaultInfo};
use crate::instructions;
use crate::{pda, KLEND_PROGRAM_ID, KVAULT_PROGRAM_ID, TOKEN_PROGRAM_ID};

/// Build a `redeem_in_kind` instruction.
///
/// Burns `shares_amount` vault shares and transfers the proportional
/// cTokens directly to the user instead of redeeming them for the
/// underlying token. PDAs and remaining accounts are derived automatically.
///
/// This is the escape hatch for when a normal `withdraw` cannot satisfy the
/// request: if the reserves backing the vault's allocations are highly
/// utilized and there isn't enough available liquidity to disinvest, the user
/// can take cTokens directly rather than wait for liquidity to free up.
///
/// If the vault has a share farm (`VaultState::vault_farm` is set), the user's
/// shares are staked in the farm and must be unstaked before this instruction
/// can burn them — see [`crate::instructions::redeem::redeem_in_kind`].
pub fn redeem_in_kind(
    vault: &VaultInfo,
    user: Pubkey,
    reserve: &ReserveInfo,
    user_ctoken_ta: Pubkey,
    user_shares_ta: Pubkey,
    shares_amount: u64,
) -> Instruction {
    let pdas = derive_vault_pdas(&vault.address);
    let (global_config, _) = pda::global_config(&KVAULT_PROGRAM_ID);
    let (ctoken_vault, _) = pda::ctoken_vault(&KVAULT_PROGRAM_ID, &vault.address, &reserve.address);
    let remaining = refresh_remaining_accounts(vault);

    instructions::redeem::redeem_in_kind(
        instructions::redeem::RedeemInKindAccounts {
            user,
            vault_state: vault.address,
            global_config,
            base_vault_authority: pdas.base_vault_authority,
            reserve: reserve.address,
            ctoken_vault,
            user_ctoken_ta,
            ctoken_mint: reserve.collateral_mint,
            user_shares_ta,
            shares_mint: pdas.shares_mint,
            reserve_collateral_token_program: TOKEN_PROGRAM_ID,
            shares_token_program: TOKEN_PROGRAM_ID,
            klend_program: KLEND_PROGRAM_ID,
        },
        shares_amount,
        remaining,
    )
}

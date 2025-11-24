use anchor_lang::prelude::*;

use crate::{KaminoVaultError, ReserveWhitelistEntry, VaultState};

#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum UpdateReserveWhitelistMode {
    Invest(u8),
    AddAllocation(u8),
}

fn check_bool_like_value(value: u8) -> Result<()> {
    if value > 1 {
        msg!("Invalid value passed in, should be 0 or 1, got {value}",);
        return Err(KaminoVaultError::InvalidBoolLikeValue.into());
    }
    Ok(())
}

pub fn update_reserve_whitelist_entry(
    reserve_whitelist_entry: &mut ReserveWhitelistEntry,
    reserve_address: &Pubkey,
    token_mint: &Pubkey,
    update: UpdateReserveWhitelistMode,
) -> Result<()> {
    reserve_whitelist_entry.reserve = *reserve_address;
    reserve_whitelist_entry.token_mint = *token_mint;

    msg!("Updating whitelisted reserve with mode {:?}", update);
    match update {
        UpdateReserveWhitelistMode::Invest(value) => {
            check_bool_like_value(value)?;
            msg!(
                "Prv value is {:?}",
                reserve_whitelist_entry.whitelist_invest
            );
            msg!("New value is {:?}", value);
            reserve_whitelist_entry.whitelist_invest = value;
        }
        UpdateReserveWhitelistMode::AddAllocation(value) => {
            check_bool_like_value(value)?;
            msg!(
                "Prv value is {:?}",
                reserve_whitelist_entry.whitelist_add_allocation
            );
            msg!("New value is {:?}", value);
            reserve_whitelist_entry.whitelist_add_allocation = value;
        }
    }

    Ok(())
}

pub fn check_can_update_allocation_weight(
    vault: &VaultState,
    reserve_idx_in_allocation: Option<usize>,
    target_allocation_weight: u64,
    allocation_cap: u64,
    reserve_whitelist_entry: Option<&ReserveWhitelistEntry>,
) -> Result<()> {
    // Get current allocation weight (0 if reserve is not in allocation)
    let (current_weight, current_cap) = match reserve_idx_in_allocation {
        Some(idx) => (
            vault.vault_allocation_strategy[idx].target_allocation_weight,
            vault.vault_allocation_strategy[idx].token_allocation_cap,
        ),
        None => (0, 0),
    };

    // Only check whitelist if target weight or allocation cap increases
    if (target_allocation_weight > current_weight || allocation_cap > current_cap)
        && vault.vault_allows_allocations_in_whitelisted_reserves_only()
    {
        let reserve_whitelist_entry =
            reserve_whitelist_entry.ok_or(KaminoVaultError::ReserveNotWhitelisted)?;
        require!(
            reserve_whitelist_entry.is_add_allocation_whitelisted(),
            KaminoVaultError::ReserveNotWhitelisted
        );
    }

    Ok(())
}

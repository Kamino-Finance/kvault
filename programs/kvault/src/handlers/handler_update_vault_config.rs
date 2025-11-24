use anchor_lang::prelude::*;
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    operations::{
        klend_operations,
        vault_config_operations::{
            self, check_if_signer_allowed_to_update_vault_config, VaultConfigField,
        },
        vault_operations::{self, common::holdings},
    },
    utils::{consts::GLOBAL_CONFIG_STATE_SEEDS, cpi_mem::CpiMemoryLender},
    GlobalConfig, VaultState,
};

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateVaultConfig<'info>>,
    entry: VaultConfigField,
    data: &[u8],
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    let global_config = ctx.accounts.global_config.load()?;
    let is_global_admin = ctx.accounts.signer.key() == global_config.global_admin;
    let is_vault_admin = ctx.accounts.signer.key() == vault.vault_admin_authority;
    check_if_signer_allowed_to_update_vault_config(&entry, data, is_global_admin, is_vault_admin)?;

    // CPI memory allocation
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );
    let reserves_count = vault.get_reserves_count();
    {
        // Refresh all reserves
        klend_operations::cpi_refresh_reserves(
            &mut cpi_mem,
            ctx.remaining_accounts.iter().take(reserves_count),
            reserves_count,
        )?;
    }
    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    let holdings = holdings(vault, reserves_iter, Clock::get()?.slot)?;
    msg!("holdings {:?}", holdings);
    // charge fees because after this the fee structure can be different
    vault_operations::charge_fees(
        vault,
        &holdings.invested,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;

    vault_config_operations::update_vault_config(vault, entry, data)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateVaultConfig<'info> {
    pub signer: Signer<'info>,

    #[account(
        seeds = [GLOBAL_CONFIG_STATE_SEEDS],
        bump,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut)]
    pub vault_state: AccountLoader<'info, VaultState>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

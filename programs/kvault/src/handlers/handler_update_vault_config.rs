use anchor_lang::prelude::*;

use crate::{
    operations::{
        vault_config_operations::{
            self, check_if_signer_allowed_to_update_vault_config, VaultConfigField,
        },
        vault_operations::{self, common::HoldingsBuffer},
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

   
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );
    let clock = Clock::get()?;

    let reserves_iter = vault_operations::common::refresh_allocation_reserve_accounts(
        &mut cpi_mem,
        vault,
        ctx.remaining_accounts,
        clock.slot,
    )?;

    let current_ts: u64 = clock.unix_timestamp.try_into().unwrap();
   
    vault_operations::refresh_rewards(vault, current_ts)?;
    let holdings = HoldingsBuffer::compute_once(vault, reserves_iter)?;
    holdings.log();
   
    vault_operations::charge_fees(vault, &holdings.invested, current_ts)?;

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

use anchor_lang::prelude::*;

use crate::{operations::vault_operations, utils::cpi_mem::CpiMemoryLender, VaultState};

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, GiveUpPendingFees<'info>>,
    max_amount_to_give_up: u64,
) -> Result<()> {
    let mut cpi_mem: CpiMemoryLender<'_> = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );

    let vault_state = &mut ctx.accounts.vault_state.load_mut()?;
    let clock = Clock::get()?;

    let reserves_iter = vault_operations::common::refresh_allocation_reserve_accounts(
        &mut cpi_mem,
        vault_state,
        ctx.remaining_accounts,
        clock.slot,
    )?;

    vault_operations::give_up_pending_fee(
        vault_state,
        reserves_iter,
        u64::try_from(clock.unix_timestamp).unwrap(),
        max_amount_to_give_up,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct GiveUpPendingFees<'info> {
    #[account(mut)]
    pub vault_admin_authority: Signer<'info>,

    #[account(mut,
        has_one = vault_admin_authority
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
}

use anchor_lang::prelude::*;
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    operations::{klend_operations, vault_operations},
    utils::cpi_mem::CpiMemoryLender,
    VaultState,
};

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
    let reserves_count = vault_state.get_reserves_count();

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

    vault_operations::give_up_pending_fee(
        vault_state,
        reserves_iter,
        clock.slot,
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

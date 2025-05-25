use anchor_lang::{prelude::*, Accounts};

use kamino_lending::Reserve;

use crate::VaultState;

pub fn process(ctx: Context<RemoveAllocation>) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;

    vault.remove_reserve_from_allocation(&ctx.accounts.reserve.key())?;

    Ok(())
}

#[derive(Accounts)]
pub struct RemoveAllocation<'info> {
    #[account(mut)]
    pub vault_admin_authority: Signer<'info>,

    #[account(mut,
        has_one = vault_admin_authority,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: check in logic if there is allocation for this reserve and it can be removed
    pub reserve: AccountLoader<'info, Reserve>,
}

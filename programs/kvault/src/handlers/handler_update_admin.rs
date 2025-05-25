use anchor_lang::prelude::*;

use crate::VaultState;

pub fn process(ctx: Context<UpdateAdmin>) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;

    msg!(
        "Update admin from {} to {}",
        vault.vault_admin_authority,
        vault.pending_admin
    );
    vault.vault_admin_authority = vault.pending_admin;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateAdmin<'info> {
    #[account(mut)]
    pub pending_admin: Signer<'info>,

    #[account(mut,
        has_one = pending_admin,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,
}

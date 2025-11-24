use anchor_lang::{prelude::*, Accounts};

use crate::{utils::consts::GLOBAL_CONFIG_STATE_SEEDS, GlobalConfig};

pub fn process(ctx: Context<UpdateGlobalConfigAdmin>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    global_config.apply_pending_admin()?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateGlobalConfigAdmin<'info> {
    pending_admin: Signer<'info>,

    #[account(mut,
        seeds = [GLOBAL_CONFIG_STATE_SEEDS],
        bump,
        has_one = pending_admin)]
    pub global_config: AccountLoader<'info, GlobalConfig>,
}

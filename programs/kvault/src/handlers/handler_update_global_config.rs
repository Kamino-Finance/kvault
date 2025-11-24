use anchor_lang::{prelude::*, Accounts};

use crate::{
    utils::{consts::GLOBAL_CONFIG_STATE_SEEDS, global_config::UpdateGlobalConfigMode},
    GlobalConfig,
};

pub fn process(ctx: Context<UpdateGlobalConfig>, update: UpdateGlobalConfigMode) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    global_config.update_value(update)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateGlobalConfig<'info> {
    global_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_STATE_SEEDS],
        bump,
        has_one = global_admin)]
    pub global_config: AccountLoader<'info, GlobalConfig>,
}

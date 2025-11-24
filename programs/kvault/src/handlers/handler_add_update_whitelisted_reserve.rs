use anchor_lang::prelude::*;
use kamino_lending::Reserve;

use crate::{
    operations::reserve_whitelist_operations::{self, UpdateReserveWhitelistMode},
    utils::consts::{
        GLOBAL_CONFIG_STATE_SEEDS, RESERVE_WHITELIST_ENTRY_SIZE, WHITELISTED_RESERVES_SEED,
    },
    xmsg, GlobalConfig, ReserveWhitelistEntry,
};

pub fn process(
    ctx: Context<AddUpdateWhitelistedReserve>,
    update: UpdateReserveWhitelistMode,
) -> Result<()> {
    let reserve_whitelist_entry = &mut ctx.accounts.reserve_whitelist_entry;
    let reserve = ctx.accounts.reserve.load()?;

    reserve_whitelist_operations::update_reserve_whitelist_entry(
        reserve_whitelist_entry,
        &ctx.accounts.reserve.key(),
        &reserve.collateral.mint_pubkey,
        update,
    )?;

    xmsg!(
        "Updated whitelisted reserve {reserve} with collateral mint {mint}",
        reserve = ctx.accounts.reserve.key(),
        mint = reserve.collateral.mint_pubkey
    );

    Ok(())
}

#[derive(Accounts)]
pub struct AddUpdateWhitelistedReserve<'info> {
    #[account(mut)]
    pub global_admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_CONFIG_STATE_SEEDS],
        bump,
        has_one = global_admin
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    pub reserve: AccountLoader<'info, Reserve>,

    #[account(
        init_if_needed,
        payer = global_admin,
        space = 8 + RESERVE_WHITELIST_ENTRY_SIZE,
        seeds = [WHITELISTED_RESERVES_SEED, reserve.key().as_ref()],
        bump
    )]
    pub reserve_whitelist_entry: Account<'info, ReserveWhitelistEntry>,

    pub system_program: Program<'info, System>,
}

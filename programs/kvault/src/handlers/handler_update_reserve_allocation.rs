use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};
use kamino_lending::Reserve;

use crate::{utils::consts::CTOKEN_VAULT_SEED, xmsg, KaminoVaultError, VaultState};

/// Update the allocation of a reserve; vault admin can insert a new reserve or update the allocation of an existing reserve, but the allocation admin can only update the allocation of existing reserves.
pub fn process(
    ctx: Context<UpdateReserveAllocation>,
    target_allocation_weight: u64,
    allocation_cap: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    let reserve = &ctx.accounts.reserve.load()?;

    let reserve_key = ctx.accounts.reserve.key();
    let idx = vault.get_reserve_idx_in_allocation(&reserve_key);
    // if the reserve is not already in allocation require the signer to be admin, otherwise the the allocation admin can also update the allocation
    let is_vault_admin = ctx.accounts.signer.key() == vault.vault_admin_authority;
    let is_allocation_admin = ctx.accounts.signer.key() == vault.allocation_admin;
    match idx {
        Some(_) => {
            require!(
                is_allocation_admin || is_vault_admin,
                KaminoVaultError::WrongAdminOrAllocationAdmin
            );
        }
        None => {
            require!(
                is_vault_admin,
                KaminoVaultError::WrongAdminOrAllocationAdmin
            );
        }
    }
    let ctoken_vault_bump = ctx.bumps.ctoken_vault;
    xmsg!(
        "Updating reserve {reserve_symbol:?} {reserve_key} with weight {target_allocation_weight} and cap {allocation_cap}",
        reserve_symbol=reserve.token_symbol(),
    );

    require_eq!(reserve.liquidity.mint_pubkey, vault.token_mint);

    vault.upsert_reserve_allocation(
        reserve_key,
        ctx.accounts.ctoken_vault.key(),
        u64::from(ctoken_vault_bump),
        target_allocation_weight,
        allocation_cap,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateReserveAllocation<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut,
        has_one = base_vault_authority
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: has_one check in the vault_state
    pub base_vault_authority: AccountInfo<'info>,

    #[account(mut,
        address = reserve.load()?.collateral.mint_pubkey,
        mint::token_program = reserve_collateral_token_program,
    )]
    pub reserve_collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Need to trust the admin
    pub reserve: AccountLoader<'info, Reserve>,

    #[account(init_if_needed,
        seeds = [CTOKEN_VAULT_SEED, vault_state.key().as_ref(), reserve.key().as_ref()],
        bump,
        payer = signer,
        token::mint = reserve_collateral_mint,
        token::authority = base_vault_authority,
        token::token_program = reserve_collateral_token_program
    )]
    pub ctoken_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub reserve_collateral_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

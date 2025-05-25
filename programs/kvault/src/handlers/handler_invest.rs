use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
    Accounts,
};
use anchor_spl::{
    token::Token,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};
use kamino_lending::{utils::FatAccountLoader, Reserve};
use token_interface::accessor::amount;

use crate::{
    kmsg,
    operations::{
        effects::{InvestEffects, InvestingDirection},
        klend_operations,
        vault_checks::{post_transfer_invest_checks, VaultBalances},
        vault_operations::{
            self,
            common::{holdings, underlying_inventory},
        },
    },
    utils::{consts::*, cpi_mem::CpiMemoryLender},
    VaultState,
};

pub fn process<'info>(ctx: Context<'_, '_, '_, 'info, Invest<'info>>) -> Result<()> {
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );

    let vault_state = &mut ctx.accounts.vault_state.load_mut()?;
    let bump = vault_state.base_vault_authority_bump;

    let reserves_count = vault_state.get_reserves_count();

    {
        // Refresh all reserves
        klend_operations::cpi_refresh_reserves(
            &mut cpi_mem,
            ctx.remaining_accounts.iter().take(reserves_count),
            reserves_count,
        )?;
    }

    let reserve = ctx.accounts.reserve.load()?;
    let reserve_address = ctx.accounts.reserve.to_account_info().key;

    let token_vault_before = amount(&ctx.accounts.token_vault.to_account_info())?;
    let ctoken_vault_before = amount(&ctx.accounts.ctoken_vault.to_account_info())?;
    let reserve_liquidity_before =
        amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;

    let Clock {
        slot: current_slot,
        unix_timestamp,
        ..
    } = Clock::get()?;
    let current_timestamp: u64 = unix_timestamp.try_into().unwrap();

    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    // Use vault_operations::invest directly which uses the holdings function internally
    let invest_effects = vault_operations::invest(
        vault_state,
        reserves_iter.clone(),
        &reserve,
        reserve_address,
        current_slot,
        current_timestamp,
    )?;

    let InvestEffects {
        direction,
        liquidity_amount,
        collateral_amount,
        rounding_loss,
    } = invest_effects;

    kmsg!(
        "InvestEffects direction={:?} liquidity_amount={}, collateral_amount={}, rounding_loss={}",
        direction,
        liquidity_amount,
        collateral_amount,
        rounding_loss
    );

    let invested_total = holdings(vault_state, reserves_iter.clone(), current_slot)?
        .invested
        .total;
    let initial_aum = vault_state.compute_aum(&invested_total)?;

    drop(reserve);

    if rounding_loss > 0 {
        // Recover the rounding loss from the crank funds
        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_interface::TransferChecked {
                    from: ctx.accounts.payer_token_account.to_account_info(),
                    to: ctx.accounts.token_vault.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                    mint: ctx.accounts.token_mint.to_account_info(),
                },
            ),
            rounding_loss,
            ctx.accounts.token_mint.decimals,
        )?;
    }

    if liquidity_amount > 0 {
        match direction {
            InvestingDirection::Add => {
                klend_operations::cpi_deposit_reserve_liquidity(
                    &ctx,
                    &mut cpi_mem,
                    bump as u8,
                    liquidity_amount,
                )?;
            }
            InvestingDirection::Subtract => {
                klend_operations::cpi_redeem_reserve_liquidity_from_invest(
                    &ctx,
                    &mut cpi_mem,
                    bump as u8,
                    collateral_amount,
                )?;
            }
        }
    }

    klend_operations::cpi_refresh_reserves(
        &mut cpi_mem,
        ctx.remaining_accounts.iter().take(reserves_count),
        reserves_count,
    )?;

    drop(cpi_mem);

    let (_, invested_after) =
        underlying_inventory(vault_state, reserves_iter.clone(), current_slot)?;
    let aum_after_invest = vault_state.compute_aum(&invested_after.total)?;

    let token_vault_after = amount(&ctx.accounts.token_vault.to_account_info())?;
    let ctoken_vault_after = amount(&ctx.accounts.ctoken_vault.to_account_info())?;
    let reserve_liquidity_after = amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;

    post_transfer_invest_checks(
        VaultBalances {
            vault_token_balance: token_vault_before,
            vault_ctoken_balance: ctoken_vault_before,
            reserve_supply_liquidity_balance: reserve_liquidity_before,
        },
        VaultBalances {
            vault_token_balance: token_vault_after,
            vault_ctoken_balance: ctoken_vault_after,
            reserve_supply_liquidity_balance: reserve_liquidity_after,
        },
        invest_effects,
        initial_aum,
        aum_after_invest,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct Invest<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut,
        token::mint = token_mint,
        token::authority = payer,
    )]
    pub payer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        has_one = base_vault_authority,
        has_one = token_vault,
        has_one = token_mint,
        has_one = token_program,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: has_one in vault_state
    #[account(mut)]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: has_one check on the vault_state
    #[account(mut)]
    pub base_vault_authority: AccountInfo<'info>,

    // Deterministic, PDA
    #[account(mut,
        seeds = [CTOKEN_VAULT_SEED, vault_state.key().as_ref(), reserve.key().as_ref()],
        bump,
        token::token_program = reserve_collateral_token_program,
    )]
    pub ctoken_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CPI accounts
    /// CHECK: check in logic if there is allocation for this reserve
    #[account(mut)]
    pub reserve: AccountLoader<'info, Reserve>,
    /// CHECK: on klend CPI call
    pub lending_market: AccountInfo<'info>,
    /// CHECK: on klend CPI call
    pub lending_market_authority: AccountInfo<'info>,
    /// CHECK: on klend CPI call
    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,
    /// CHECK: on klend CPI call
    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    pub reserve_collateral_token_program: Program<'info, Token>,
    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Syvar Instruction allowing introspection, fixed address
    #[account(address = SysInstructions::id())]
    pub instruction_sysvar_account: AccountInfo<'info>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

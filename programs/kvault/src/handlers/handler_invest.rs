use anchor_lang::{
    prelude::*,
    solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId},
    Accounts,
};
use anchor_spl::{
    token::Token,
    token_interface::{self, accessor::amount, Mint, TokenAccount, TokenInterface},
};
use kamino_lending::Reserve;

use crate::{
    kmsg,
    operations::{
        effects::{InvestEffects, InvestingDirection},
        klend_operations,
        vault_checks::{post_transfer_invest_checks, VaultBalances},
        vault_operations::{self, common::HoldingsBuffer},
    },
    utils::{consts::*, cpi_mem::CpiMemoryLender},
    ReserveWhitelistEntry, VaultState,
};






pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, Invest<'info>>,
    max_amount: u64,
) -> Result<()> {
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender_with_max_data(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
        klend_operations::KLEND_CPI_U64_ARG_DATA_LEN,
    );

    let vault_state = &mut ctx.accounts.vault_state.load_mut()?;
    let bump = vault_state.base_vault_authority_bump;
    let reserves_count = vault_state.get_reserves_count();
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    let current_timestamp: u64 = clock.unix_timestamp.try_into().unwrap();

    let reserves_iter = vault_operations::common::refresh_allocation_reserve_accounts(
        &mut cpi_mem,
        vault_state,
        ctx.remaining_accounts,
        current_slot,
    )?;

    let reserve = ctx.accounts.reserve.load()?;
    let reserve_address = ctx.accounts.reserve.to_account_info().key;

    let token_vault_before = amount(&ctx.accounts.token_vault.to_account_info())?;
    let ctoken_vault_before = amount(&ctx.accounts.ctoken_vault.to_account_info())?;
    let reserve_liquidity_before =
        amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;

    vault_operations::refresh_rewards(vault_state, current_timestamp)?;
    let mut holdings_buffer = HoldingsBuffer::new();
    let mut holdings = holdings_buffer.compute(vault_state, reserves_iter.clone())?;
    let initial_holdings_total = holdings.total_sum;

    let invest_effects = vault_operations::invest_with_holdings_snapshot(
        vault_state,
        &reserve,
        reserve_address,
        current_slot,
        current_timestamp,
        ctx.accounts
            .reserve_whitelist_entry
            .as_ref()
            .map(|acc| acc.as_ref()),
        max_amount,
        &holdings,
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

   
   
   
   
    let pre_transfer_holdings = holdings.sync_reserve(vault_state, reserve_address, &reserve)?;
    let aum_before_transfers = pre_transfer_holdings.aum;

    drop(reserve);

    if rounding_loss > 0 {
       
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

    let refreshed_reserve = ctx.accounts.reserve.load()?;
   
   
   
    let post_transfer_holdings =
        holdings.sync_reserve(vault_state, reserve_address, &refreshed_reserve)?;
    drop(refreshed_reserve);

    let aum_after_transfers = post_transfer_holdings.aum;
    let final_holdings_total = post_transfer_holdings.total_sum;

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
        initial_holdings_total,
        final_holdings_total,
        aum_before_transfers,
        aum_after_transfers,
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

    #[account(
        seeds = [WHITELISTED_RESERVES_SEED, reserve.key().as_ref()],
        bump
    )]
    pub reserve_whitelist_entry: Option<Account<'info, ReserveWhitelistEntry>>,

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

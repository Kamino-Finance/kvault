use std::convert::TryFrom;

use anchor_lang::{prelude::*, solana_program::sysvar};
use anchor_spl::{
    token::Token,
    token_interface::{accessor::amount, TokenAccount, TokenInterface},
};
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    operations::{
        effects::WithdrawPendingFeesEffects,
        klend_operations,
        vault_checks::{post_transfer_withdraw_pending_fees_balance_checks, VaultAndUserBalances},
        vault_operations,
    },
    utils::{consts::CTOKEN_VAULT_SEED, cpi_mem::CpiMemoryLender, token_ops},
    KaminoVaultError, VaultState,
};

pub fn process<'info>(ctx: Context<'_, '_, '_, 'info, WithdrawPendingFees<'info>>) -> Result<()> {
    let mut cpi_mem = CpiMemoryLender::build_cpi_memory_lender(
        ctx.accounts.to_account_infos(),
        ctx.remaining_accounts,
    );

    let vault_state = &mut ctx.accounts.vault_state.load_mut()?;
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
    let bump = vault_state.base_vault_authority_bump;
    let reserve_address = ctx.accounts.reserve.to_account_info().key;

    // Cache some values
    let token_vault_before = ctx.accounts.token_vault.amount;
    let ctoken_vault_before = ctx.accounts.ctoken_vault.amount;
    let admin_ata_before = ctx.accounts.token_ata.amount;
    let reserve_supply_liquidity_before = ctx.accounts.reserve_liquidity_supply.amount;

    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    let reserve_allocation = vault_state.allocation_for_reserve(reserve_address)?;
    require_keys_eq!(
        reserve_allocation.ctoken_vault,
        ctx.accounts.ctoken_vault.key()
    );

    let withdraw_pending_fees_effects = {
        vault_operations::withdraw_pending_fees(
            vault_state,
            reserve_address,
            &reserve,
            reserves_iter,
            Clock::get()?.slot,
            Clock::get()?.unix_timestamp.try_into().unwrap(),
        )?
    };

    let WithdrawPendingFeesEffects {
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest,
    } = withdraw_pending_fees_effects;

    msg!("WithdrawPendingFeesEffects: available_to_send_to_user={}, invested_to_disinvest_ctokens={}, invested_liquidity_to_send_to_user={}, invested_liquidity_to_disinvest={}",
        available_to_send_to_user,
        invested_to_disinvest_ctokens,
        invested_liquidity_to_send_to_user,
        invested_liquidity_to_disinvest
    );

    drop(reserve);

    //1. Disinvest from reserve to the kvault token vault
    if invested_to_disinvest_ctokens > 0 {
        klend_operations::cpi_redeem_reserve_liquidity_from_withdraw_pending_fees(
            &ctx,
            &mut cpi_mem,
            bump as u8,
            invested_to_disinvest_ctokens,
        )?;
    }

    let token_vault_before_transfer = amount(&ctx.accounts.token_vault.to_account_info())?;
    let liquidity_received = token_vault_before_transfer - token_vault_before;

    require!(
        liquidity_received >= invested_liquidity_to_send_to_user,
        KaminoVaultError::NotEnoughLiquidityDisinvestedToSendToUser
    );

    // 2. Send all the owed tokens to the admin
    token_ops::tokens::transfer_to_token_account(
        &token_ops::tokens::VaultTransferAccounts {
            token_program: ctx.accounts.token_program.to_account_info(),
            token_vault: ctx.accounts.token_vault.to_account_info(),
            token_ata: ctx.accounts.token_ata.to_account_info(),
            token_mint: ctx.accounts.token_mint.to_account_info(),
            base_vault_authority: ctx.accounts.base_vault_authority.to_account_info(),
            vault_state: ctx.accounts.vault_state.to_account_info(),
        },
        u8::try_from(vault_state.base_vault_authority_bump).unwrap(),
        available_to_send_to_user + invested_liquidity_to_send_to_user,
        u8::try_from(vault_state.token_mint_decimals).unwrap(),
    )?;

    // Post checks
    let token_vault_after = amount(&ctx.accounts.token_vault.to_account_info())?;
    let ctoken_vault_after = amount(&ctx.accounts.ctoken_vault.to_account_info())?;
    let admin_ata_after = amount(&ctx.accounts.token_ata.to_account_info())?;
    let reserve_supply_liquidity_after =
        amount(&ctx.accounts.reserve_liquidity_supply.to_account_info())?;

    post_transfer_withdraw_pending_fees_balance_checks(
        VaultAndUserBalances {
            reserve_supply_liquidity_balance: reserve_supply_liquidity_before,
            vault_token_balance: token_vault_before,
            vault_ctoken_balance: ctoken_vault_before,
            user_token_balance: admin_ata_before,
            user_shares_balance: 0, // placeholder, we don't use shares
        },
        VaultAndUserBalances {
            reserve_supply_liquidity_balance: reserve_supply_liquidity_after,
            vault_token_balance: token_vault_after,
            vault_ctoken_balance: ctoken_vault_after,
            user_token_balance: admin_ata_after,
            user_shares_balance: 0, // placeholder, we don't use shares
        },
        withdraw_pending_fees_effects,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawPendingFees<'info> {
    #[account(mut)]
    pub vault_admin_authority: Signer<'info>,

    #[account(mut,
        has_one = base_vault_authority,
        has_one = token_vault,
        has_one = token_mint,
        has_one = token_program,
        has_one = vault_admin_authority
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: check in logic if there is allocation for this reserve
    #[account(mut)]
    pub reserve: AccountLoader<'info, Reserve>,

    #[account(mut,
        token::token_program = token_program,
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // Deterministic, PDA
    #[account(mut,
        seeds = [CTOKEN_VAULT_SEED, vault_state.key().as_ref(), reserve.key().as_ref()],
        bump,
        token::token_program = reserve_collateral_token_program,
    )]
    pub ctoken_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: This authority is stored in the vault state
    #[account(mut)]
    pub base_vault_authority: AccountInfo<'info>,

    /// CHECK: the fields of the associated_token account are stored in the vault state
    #[account(mut,
        associated_token::mint = token_mint,
        associated_token::authority = vault_admin_authority,
        associated_token::token_program = token_program,
    )]
    pub token_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: has_one in the vault state
    #[account(mut)]
    pub token_mint: AccountInfo<'info>,

    /// CPI accounts
    /// CHECK: The account is checked on CPI calls
    pub lending_market: AccountInfo<'info>,
    /// CHECK: The account is checked on CPI calls
    pub lending_market_authority: AccountInfo<'info>,
    #[account(mut)]
    pub reserve_liquidity_supply: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: The account is checked on CPI calls
    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    pub token_program: Interface<'info, TokenInterface>,
    pub reserve_collateral_token_program: Program<'info, Token>,

    /// CHECK: account constraints checked in account trait
    #[account(address = sysvar::instructions::ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

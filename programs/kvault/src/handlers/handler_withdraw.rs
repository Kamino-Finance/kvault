use std::convert::TryFrom;

use anchor_lang::{prelude::*, solana_program::sysvar};
use anchor_spl::{
    token::Token,
    token_interface::{accessor::amount, Mint, TokenAccount, TokenInterface},
};
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    operations::{
        effects::WithdrawEffects,
        klend_operations,
        vault_checks::{post_transfer_withdraw_balance_checks, VaultAndUserBalances},
        vault_operations,
    },
    utils::{
        consts::CTOKEN_VAULT_SEED,
        cpi_mem::CpiMemoryLender,
        token_ops::{self, shares},
    },
    KaminoVaultError, VaultState,
};

pub fn withdraw<'info>(
    ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>,
    shares_amount: u64,
) -> Result<()> {
    let withdraw_from_available = &ctx.accounts.withdraw_from_available;
    let withdraw_from_reserve = &ctx.accounts.withdraw_from_reserve_accounts;

    require_keys_eq!(
        withdraw_from_available.vault_state.key(),
        withdraw_from_reserve.vault_state.key()
    );

    let (shares_to_withdraw_event, withdraw_result_event) = withdraw_utils::withdraw(
        withdraw_from_available,
        Some(withdraw_from_reserve),
        ctx.remaining_accounts,
        shares_amount,
    )?;

    emit_cpi!(shares_to_withdraw_event);
    emit_cpi!(withdraw_result_event);

    Ok(())
}

pub fn withdraw_from_available<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawFromAvailable<'info>>,
    shares_amount: u64,
) -> Result<()> {
    let (shares_to_withdraw_event, withdraw_result_event) =
        withdraw_utils::withdraw(ctx.accounts, None, ctx.remaining_accounts, shares_amount)?;

    emit_cpi!(shares_to_withdraw_event);
    emit_cpi!(withdraw_result_event);

    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub withdraw_from_available: WithdrawFromAvailable<'info>,

    /// CPI accounts
    pub withdraw_from_reserve_accounts: WithdrawFromInvested<'info>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

#[derive(Accounts)]
pub struct WithdrawFromInvested<'info> {
    #[account(mut)]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: check in logic if there is allocation for this reserve
    #[account(mut)]
    pub reserve: AccountLoader<'info, Reserve>,

    // Deterministic, PDA
    #[account(mut,
            seeds = [CTOKEN_VAULT_SEED, vault_state.key().as_ref(), reserve.key().as_ref()],
            bump,
            token::token_program = reserve_collateral_token_program,
        )]
    pub ctoken_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    #[account(mut)]
    pub reserve_liquidity_supply: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    pub reserve_collateral_token_program: Program<'info, Token>,

    /// CHECK: account constraints checked in account trait
    #[account(address = sysvar::instructions::ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct WithdrawFromAvailable<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut,
        has_one = base_vault_authority,
        has_one = token_vault,
        has_one = token_mint,
        has_one = token_program,
        has_one = shares_mint,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    #[account(mut,
        token::token_program = token_program,
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: has_one check in vault_state
    pub base_vault_authority: AccountInfo<'info>,

    /// CHECK: vault_state checks the token mint and the token program
    #[account(mut,
        token::mint = token_mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_token_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: has_one check on the vault state account
    #[account(mut)]
    pub token_mint: AccountInfo<'info>,

    #[account(mut,
        token::mint = shares_mint,
        token::authority = user,
        token::token_program = shares_token_program
    )]
    pub user_shares_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub shares_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub shares_token_program: Program<'info, Token>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    // For withdraw from available this context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

pub mod withdraw_utils {
    use crate::events::{SharesToWithdrawEvent, WithdrawResultEvent};

    use super::*;

    pub fn withdraw<'info>(
        ctx_withdraw_from_available: &WithdrawFromAvailable<'info>,
        ctx_withdraw_from_reserves: Option<&WithdrawFromInvested<'info>>,
        remaining_accounts: &[AccountInfo<'info>],
        shares_amount: u64,
    ) -> Result<(SharesToWithdrawEvent, WithdrawResultEvent)> {
        let withdraw_from_available_accounts = ctx_withdraw_from_available;

        let should_withdraw_from_invested = ctx_withdraw_from_reserves.is_some();

        let mut all_accounts = withdraw_from_available_accounts.to_account_infos();
        if should_withdraw_from_invested {
            all_accounts.extend_from_slice(&ctx_withdraw_from_reserves.unwrap().to_account_infos());
        }

        let mut cpi_mem =
            CpiMemoryLender::build_cpi_memory_lender(all_accounts, remaining_accounts);

        let vault_state: &mut std::cell::RefMut<'_, VaultState> =
            &mut withdraw_from_available_accounts.vault_state.load_mut()?;
        let reserves_count = vault_state.get_reserves_count();

        // Cache some values for withdraw from available
        let token_vault_before = withdraw_from_available_accounts.token_vault.amount;
        let user_ata_before = withdraw_from_available_accounts.user_token_ata.amount;
        let user_shares_before = withdraw_from_available_accounts.user_shares_ata.amount;

        let (ctoken_vault_before, reserve_supply_liquidity_before) =
            if should_withdraw_from_invested {
                let withdraw_from_reserve_accounts = &ctx_withdraw_from_reserves.unwrap();
                let ctoken_vault_before = withdraw_from_reserve_accounts.ctoken_vault.amount;
                let reserve_supply_liquidity_before = withdraw_from_reserve_accounts
                    .reserve_liquidity_supply
                    .amount;
                (ctoken_vault_before, reserve_supply_liquidity_before)
            } else {
                (0, 0)
            };

        // if the user asks to withdraw more shares than they have, withdraw all the shares they have
        let shares_amount = std::cmp::min(shares_amount, user_shares_before);
        let shares_to_withdraw_event = SharesToWithdrawEvent {
            shares_amount,
            user_shares_before,
        };

        klend_operations::cpi_refresh_reserves(
            &mut cpi_mem,
            remaining_accounts.iter().take(reserves_count),
            reserves_count,
        )?;

        let reserves_iter = remaining_accounts
            .iter()
            .take(reserves_count)
            .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

        let (reserve_address_to_withdraw_from, reserve_state_to_withdraw_from, ctokens): (_, _, _) =
            if should_withdraw_from_invested {
                let withdraw_from_reserve_accounts = &ctx_withdraw_from_reserves.unwrap();

                let reserve = withdraw_from_reserve_accounts.reserve.load()?;
                let reserve_address = withdraw_from_reserve_accounts.reserve.to_account_info().key;

                let reserve_allocation = vault_state
                    .allocation_for_reserve(&withdraw_from_reserve_accounts.reserve.key())?;
                require_keys_eq!(
                    reserve_allocation.ctoken_vault,
                    withdraw_from_reserve_accounts.ctoken_vault.key()
                );

                (
                    Some(reserve_address),
                    Some(reserve),
                    Some(reserve_allocation.ctoken_allocation),
                )
            } else {
                (None, None, None)
            };

        let withdraw_effects = vault_operations::withdraw(
            vault_state,
            reserve_address_to_withdraw_from,
            reserve_state_to_withdraw_from.as_deref(),
            reserves_iter,
            Clock::get()?.unix_timestamp.try_into().unwrap(),
            Clock::get()?.slot,
            shares_amount,
            ctokens,
        )?;

        let WithdrawEffects {
            shares_to_burn,
            available_to_send_to_user,
            invested_to_disinvest_ctokens,
            invested_liquidity_to_send_to_user,
            invested_liquidity_to_disinvest: _,
        } = withdraw_effects;

        let withdraw_result_event = WithdrawResultEvent {
            shares_to_burn,
            available_to_send_to_user,
            invested_to_disinvest_ctokens,
            invested_liquidity_to_send_to_user,
        };

        drop(reserve_state_to_withdraw_from);

        // 1. Burn shares
        shares::burn(
            withdraw_from_available_accounts
                .shares_mint
                .to_account_info(),
            withdraw_from_available_accounts
                .user_shares_ata
                .to_account_info(),
            withdraw_from_available_accounts.user.to_account_info(),
            withdraw_from_available_accounts
                .shares_token_program
                .to_account_info(),
            shares_to_burn,
        )?;

        // 2. Disinvest from reserve to the kvault token vault
        if invested_to_disinvest_ctokens > 0 {
            klend_operations::cpi_redeem_reserve_liquidity_from_withdraw(
                ctx_withdraw_from_available,
                ctx_withdraw_from_reserves.unwrap(),
                &mut cpi_mem,
                vault_state.base_vault_authority_bump as u8,
                invested_to_disinvest_ctokens,
            )?;
        }

        let token_vault_before_transfer_to_user = amount(
            &withdraw_from_available_accounts
                .token_vault
                .to_account_info(),
        )?;
        let liquidity_received = token_vault_before_transfer_to_user - token_vault_before;

        require!(
            liquidity_received >= invested_liquidity_to_send_to_user,
            KaminoVaultError::NotEnoughLiquidityDisinvestedToSendToUser
        );

        // 3. Send all the owed tokens to user
        token_ops::tokens::transfer_to_token_account(
            &token_ops::tokens::VaultTransferAccounts {
                token_program: withdraw_from_available_accounts
                    .token_program
                    .to_account_info(),
                token_vault: withdraw_from_available_accounts
                    .token_vault
                    .to_account_info(),
                token_ata: withdraw_from_available_accounts
                    .user_token_ata
                    .to_account_info(),
                token_mint: withdraw_from_available_accounts
                    .token_mint
                    .to_account_info(),
                base_vault_authority: withdraw_from_available_accounts
                    .base_vault_authority
                    .to_account_info(),
                vault_state: withdraw_from_available_accounts
                    .vault_state
                    .to_account_info(),
            },
            u8::try_from(vault_state.base_vault_authority_bump).unwrap(),
            available_to_send_to_user + invested_liquidity_to_send_to_user,
            u8::try_from(vault_state.token_mint_decimals).unwrap(),
        )?;

        let token_vault_after = amount(
            &withdraw_from_available_accounts
                .token_vault
                .to_account_info(),
        )?;
        let user_ata_after = amount(
            &withdraw_from_available_accounts
                .user_token_ata
                .to_account_info(),
        )?;
        let user_shares_after = amount(
            &withdraw_from_available_accounts
                .user_shares_ata
                .to_account_info(),
        )?;

        let (ctoken_vault_after, reserve_supply_liquidity_after) = if should_withdraw_from_invested
        {
            let withdraw_from_reserve_accounts = &ctx_withdraw_from_reserves.unwrap();
            let ctoken_vault_after = amount(
                &withdraw_from_reserve_accounts
                    .ctoken_vault
                    .to_account_info(),
            )?;
            let reserve_supply_liquidity_after = amount(
                &withdraw_from_reserve_accounts
                    .reserve_liquidity_supply
                    .to_account_info(),
            )?;
            (ctoken_vault_after, reserve_supply_liquidity_after)
        } else {
            (0, 0)
        };

        // Post checks
        post_transfer_withdraw_balance_checks(
            VaultAndUserBalances {
                reserve_supply_liquidity_balance: reserve_supply_liquidity_before,
                vault_token_balance: token_vault_before,
                vault_ctoken_balance: ctoken_vault_before,
                user_token_balance: user_ata_before,
                user_shares_balance: user_shares_before,
            },
            VaultAndUserBalances {
                reserve_supply_liquidity_balance: reserve_supply_liquidity_after,
                vault_token_balance: token_vault_after,
                vault_ctoken_balance: ctoken_vault_after,
                user_token_balance: user_ata_after,
                user_shares_balance: user_shares_after,
            },
            withdraw_effects,
        )?;

        Ok((shares_to_withdraw_event, withdraw_result_event))
    }
}

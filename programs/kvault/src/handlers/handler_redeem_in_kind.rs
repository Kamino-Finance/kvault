use std::convert::TryFrom;

use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_interface::{accessor::amount, Mint, TokenAccount},
};
use kamino_lending::{
    fraction::Fraction,
    utils::{AnyAccountLoader, FatAccountLoader},
    Reserve,
};
use solana_program::clock::Slot;

use crate::{
    events::{RedeemInKindResultEvent, SharesToWithdrawEvent},
    operations::{
        effects::RedeemInKindEffects,
        klend_operations,
        vault_checks::{post_redeem_in_kind_checks, RedeemInKindPostCheckAmounts},
        vault_operations::{self, common::underlying_inventory},
    },
    utils::{
        consts::{CTOKEN_VAULT_SEED, GLOBAL_CONFIG_STATE_SEEDS},
        cpi_mem::CpiMemoryLender,
        token_ops::{shares, tokens},
    },
    GlobalConfig, VaultState,
};

pub fn redeem_in_kind<'info>(
    ctx: Context<'_, '_, '_, 'info, RedeemInKind<'info>>,
    shares_amount: u64,
) -> Result<()> {
    let all_accounts = ctx.accounts.to_account_infos();

    let mut cpi_mem =
        CpiMemoryLender::build_cpi_memory_lender(all_accounts, ctx.remaining_accounts);

    let vault_state = &mut ctx.accounts.vault_state.load_mut()?;
    let global_config = &ctx.accounts.global_config.load()?;
    let reserves_count = vault_state.get_reserves_count();
    let clock = Clock::get()?;
    let current_slot = clock.slot;

    let make_reserves_iter = || {
        ctx.remaining_accounts
            .iter()
            .take(reserves_count)
            .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap())
    };

    klend_operations::cpi_refresh_reserves(
        &mut cpi_mem,
        ctx.remaining_accounts.iter().take(reserves_count),
        reserves_count,
    )?;

   
    let user_shares_balance = ctx.accounts.user_shares_ta.amount;
    let shares_amount = shares_amount.min(user_shares_balance);

    emit_cpi!(SharesToWithdrawEvent {
        shares_amount,
        user_shares_before: user_shares_balance,
    });

    let reserve = ctx.accounts.reserve.load()?;
    let reserve_address = &ctx.accounts.reserve.key();

    let RedeemInKindEffects {
        shares_to_burn,
        ctokens_to_send_to_user,
        actual_liquidity_value,
        vault_aum_before,
    } = vault_operations::redeem_in_kind(vault_operations::RedeemInKindParams {
        vault_state,
        global_config,
        reserve_address,
        reserve_state: &reserve,
        reserves_iter: make_reserves_iter(),
        shares_amount,
        clock: &clock,
    })?;

   
    let amounts_before = collect_post_check_amounts(ctx.accounts, &vault_aum_before)?;

    emit_cpi!(RedeemInKindResultEvent {
        shares_to_burn,
        ctokens_to_send_to_user,
    });

   
    shares::burn(
        ctx.accounts.shares_mint.to_account_info(),
        ctx.accounts.user_shares_ta.to_account_info(),
        ctx.accounts.user.to_account_info(),
        ctx.accounts.shares_token_program.to_account_info(),
        shares_to_burn,
    )?;

   
    tokens::transfer_to_token_account(
        &tokens::VaultTransferAccounts {
            token_program: ctx
                .accounts
                .reserve_collateral_token_program
                .to_account_info(),
            token_vault: ctx.accounts.ctoken_vault.to_account_info(),
            token_ata: ctx.accounts.user_ctoken_ta.to_account_info(),
            token_mint: ctx.accounts.ctoken_mint.to_account_info(),
            base_vault_authority: ctx.accounts.base_vault_authority.to_account_info(),
            vault_state: ctx.accounts.vault_state.to_account_info(),
        },
        u8::try_from(vault_state.base_vault_authority_bump).unwrap(),
        ctokens_to_send_to_user,
        ctx.accounts.ctoken_mint.decimals,
    )?;

    let vault_aum_after = calculate_vault_aum(vault_state, make_reserves_iter(), current_slot)?;
    let amounts_after = collect_post_check_amounts(ctx.accounts, &vault_aum_after)?;
    post_redeem_in_kind_checks(
        &amounts_before,
        &amounts_after,
        ctokens_to_send_to_user,
        shares_to_burn,
        actual_liquidity_value,
    )?;

    Ok(())
}

fn collect_post_check_amounts(
    accounts: &RedeemInKind,
    vault_aum: &Fraction,
) -> Result<RedeemInKindPostCheckAmounts> {
   
   
   
    Ok(RedeemInKindPostCheckAmounts {
        user_share_balance: amount(&accounts.user_shares_ta.to_account_info())?,
        vault_ctoken_balance: amount(&accounts.ctoken_vault.to_account_info())?,
        user_ctoken_balance: amount(&accounts.user_ctoken_ta.to_account_info())?,
        vault_aum: *vault_aum,
    })
}


fn calculate_vault_aum<'a>(
    vault_state: &VaultState,
    reserves_iter: impl Iterator<Item = impl AnyAccountLoader<'a, Reserve>>,
    current_slot: Slot,
) -> Result<Fraction> {
    let (_, invested) = underlying_inventory(vault_state, reserves_iter, current_slot)?;
    let vault_aum = vault_state.compute_aum(&invested.total)?;
    Ok(vault_aum)
}

#[event_cpi]
#[derive(Accounts)]
pub struct RedeemInKind<'info> {
    pub user: Signer<'info>,

    #[account(mut,
        has_one = base_vault_authority,
        has_one = shares_mint,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    #[account(
        seeds = [GLOBAL_CONFIG_STATE_SEEDS],
        bump,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    /// CHECK: has_one check in vault_state
    pub base_vault_authority: AccountInfo<'info>,

    /// CHECK: check in logic if there is allocation for this reserve
    #[account(mut)]
    pub reserve: AccountLoader<'info, Reserve>,

    // Deterministic, PDA
    #[account(mut,
        seeds = [CTOKEN_VAULT_SEED, vault_state.key().as_ref(), reserve.key().as_ref()],
        bump,
        token::mint = ctoken_mint,
        token::token_program = reserve_collateral_token_program,
    )]
    pub ctoken_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        token::mint = ctoken_mint,
        token::authority = user,
        token::token_program = reserve_collateral_token_program,
    )]
    pub user_ctoken_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        address = reserve.load()?.collateral.mint_pubkey
    )]
    pub ctoken_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        token::mint = shares_mint,
        token::authority = user,
        token::token_program = shares_token_program,
    )]
    pub user_shares_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub shares_mint: Box<InterfaceAccount<'info, Mint>>,

    pub reserve_collateral_token_program: Program<'info, Token>,
    pub shares_token_program: Program<'info, Token>,
    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    // This context has remaining accounts:
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

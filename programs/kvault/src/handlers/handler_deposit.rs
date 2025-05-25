use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    token::{accessor::amount, Token},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    events::{DepositResultEvent, DepositUserAtaBalanceEvent},
    operations::{effects::DepositEffects, klend_operations, vault_operations},
    utils::{
        cpi_mem::CpiMemoryLender,
        token_ops::{self, shares, tokens::UserTransferAccounts},
    },
    KaminoVaultError, VaultState,
};

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, Deposit<'info>>,
    max_amount: u64,
) -> Result<()> {
    // CHECKS
    require!(max_amount > 0, KaminoVaultError::DepositAmountsZero);

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

    let user_initial_shares_balance = ctx.accounts.user_shares_ata.amount;
    let user_intial_ata_balance = ctx.accounts.user_token_ata.amount;
    let initial_vault_shares_issued = vault_state.shares_issued;
    emit_cpi!(DepositUserAtaBalanceEvent {
        user_ata_balance: user_intial_ata_balance,
    });

    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .take(reserves_count)
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    let DepositEffects {
        shares_to_mint,
        token_to_deposit,
        crank_funds_to_deposit,
    } = vault_operations::deposit(
        vault_state,
        reserves_iter,
        max_amount,
        Clock::get()?.slot,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;
    emit_cpi!(DepositResultEvent {
        shares_to_mint,
        token_to_deposit,
        crank_funds_to_deposit,
    });

    // Deposit from user token
    token_ops::tokens::transfer_to_vault(
        &UserTransferAccounts {
            token_program: ctx.accounts.token_program.to_account_info(),
            user_authority: ctx.accounts.user.to_account_info(),
            token_ata: ctx.accounts.user_token_ata.to_account_info(),
            token_vault: ctx.accounts.token_vault.to_account_info(),
            token_mint: ctx.accounts.token_mint.to_account_info(),
        },
        token_to_deposit + crank_funds_to_deposit,
        ctx.accounts.token_mint.decimals,
    )?;

    // Mint shares
    shares::mint(
        ctx.accounts.shares_token_program.to_account_info(),
        ctx.accounts.shares_mint.to_account_info(),
        ctx.accounts.vault_state.to_account_info(),
        ctx.accounts.base_vault_authority.to_account_info(),
        ctx.accounts.user_shares_ata.to_account_info(),
        vault_state.base_vault_authority_bump,
        shares_to_mint,
    )?;

    // Post checks
    let user_ata_balance_after = amount(&ctx.accounts.user_token_ata.to_account_info())?;
    let user_shares_balance_after = amount(&ctx.accounts.user_shares_ata.to_account_info())?;
    let user_shares_gained = user_shares_balance_after - user_initial_shares_balance;

    require!(
        initial_vault_shares_issued + user_shares_gained == vault_state.shares_issued,
        KaminoVaultError::SharesIssuedAmountDoesNotMatch,
    );

    require!(
        user_intial_ata_balance - token_to_deposit - crank_funds_to_deposit
            == user_ata_balance_after,
        KaminoVaultError::TokensDepositedAmountDoesNotMatch,
    );

    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct Deposit<'info> {
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

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // The base token of the vault
    /// CHECK: vault_state has_one check
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: vault_state has_one check
    pub base_vault_authority: AccountInfo<'info>,

    /// CHECK: vault_state has_one check
    #[account(mut,
        mint::token_program = shares_token_program
    )]
    pub shares_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        token::mint = token_mint,
        token::authority = user
    )]
    pub user_token_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        token::mint = shares_mint,
        token::authority = user
    )]
    pub user_shares_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub klend_program: Program<'info, kamino_lending::program::KaminoLending>,
    pub token_program: Interface<'info, TokenInterface>,
    pub shares_token_program: Program<'info, Token>,
    // This context (list of accounts) has a lot of remaining accounts,
    // - All reserves entries of this vault
    // - All of the associated lending market accounts
    // They are dynamically sized and ordered and cannot be declared here upfront
}

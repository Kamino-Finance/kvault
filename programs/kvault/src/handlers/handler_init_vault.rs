use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use kamino_lending::{utils::FatAccountLoader, Reserve};

use crate::{
    operations::{effects::DepositEffects, vault_operations},
    utils::{
        consts::*,
        token_ops::{self, tokens::UserTransferAccounts},
    },
    VaultState,
};

pub fn process(ctx: Context<InitVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_init()?;

    vault.vault_admin_authority = ctx.accounts.admin_authority.key();
    vault.allocation_admin = ctx.accounts.admin_authority.key();
    vault.token_mint = ctx.accounts.base_token_mint.key();
    vault.token_vault = ctx.accounts.token_vault.key();
    vault.token_program = ctx.accounts.token_program.key();
    vault.base_vault_authority = ctx.accounts.base_vault_authority.key();
    vault.shares_mint = ctx.accounts.shares_mint.key();
    vault.base_vault_authority_bump = u64::from(ctx.bumps.base_vault_authority);

    let clock = &Clock::get()?;
    vault_operations::initialize(
        vault,
        ctx.accounts.base_token_mint.decimals,
        ctx.accounts.shares_mint.decimals,
        u64::try_from(clock.unix_timestamp).unwrap(),
    )?;

    let reserves_iter = ctx
        .remaining_accounts
        .iter()
        .map(|account_info| FatAccountLoader::<Reserve>::try_from(account_info).unwrap());

    // deposit INITIAL_DEPOSIT_AMOUNT to set the share price and avoid price inflation of initial shares
    let DepositEffects {
        shares_to_mint,
        token_to_deposit,
        crank_funds_to_deposit,
    } = vault_operations::deposit(
        vault,
        reserves_iter,
        INITIAL_DEPOSIT_AMOUNT,
        clock.slot,
        clock.unix_timestamp.try_into().unwrap(),
    )?;

    msg!(
        "shares_on_init={} token_to_deposit={} crank_funds_to_deposit={}",
        shares_to_mint,
        token_to_deposit,
        crank_funds_to_deposit
    );

    // Deposit from admin token
    token_ops::tokens::transfer_to_vault(
        &UserTransferAccounts {
            token_program: ctx.accounts.token_program.to_account_info(),
            user_authority: ctx.accounts.admin_authority.to_account_info(),
            token_ata: ctx.accounts.admin_token_account.to_account_info(),
            token_vault: ctx.accounts.token_vault.to_account_info(),
            token_mint: ctx.accounts.base_token_mint.to_account_info(),
        },
        token_to_deposit + crank_funds_to_deposit,
        ctx.accounts.base_token_mint.decimals,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub admin_authority: Signer<'info>,

    #[account(zero)]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: PDA owned by the program
    #[account(seeds = [BASE_VAULT_AUTHORITY_SEED, vault_state.key().as_ref()], bump)]
    pub base_vault_authority: AccountInfo<'info>,

    #[account(init,
        seeds = [TOKEN_VAULT_SEED, vault_state.key().as_ref()],
        bump,
        payer = admin_authority,
        token::mint = base_token_mint,
        token::authority = base_vault_authority,
        token::token_program = token_program,
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // The base token of the vault
    #[account(
            mint::token_program = token_program,
        )]
    pub base_token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(init,
        seeds=[SHARES_SEEDS, vault_state.key().as_ref()],
        bump,
        payer = admin_authority,
        mint::decimals = base_token_mint.decimals,
        mint::authority = base_vault_authority,
        mint::token_program = shares_token_program,
    )]
    pub shares_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        token::mint = base_token_mint,
        token::authority = admin_authority
    )]
    pub admin_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Interface<'info, TokenInterface>,
    pub shares_token_program: Program<'info, Token>,
}

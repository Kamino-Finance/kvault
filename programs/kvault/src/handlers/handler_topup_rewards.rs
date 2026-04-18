use anchor_lang::prelude::*;
use anchor_spl::token_interface::{accessor, Mint, TokenAccount, TokenInterface};

use crate::{
    operations::vault_operations, utils::token_ops::tokens::UserTransferAccounts, KaminoVaultError,
    VaultState,
};

pub fn process(ctx: Context<TopupRewards>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    let current_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    vault_operations::topup_rewards(vault, amount, current_ts)?;

    let initial_payer_token_balance =
        accessor::amount(&ctx.accounts.payer_token_ta.to_account_info())?;
    let initial_vault_token_balance =
        accessor::amount(&ctx.accounts.token_vault.to_account_info())?;

    crate::utils::token_ops::tokens::transfer_to_vault(
        &UserTransferAccounts {
            token_program: ctx.accounts.token_program.to_account_info(),
            user_authority: ctx.accounts.payer.to_account_info(),
            token_ata: ctx.accounts.payer_token_ta.to_account_info(),
            token_vault: ctx.accounts.token_vault.to_account_info(),
            token_mint: ctx.accounts.token_mint.to_account_info(),
        },
        amount,
        ctx.accounts.token_mint.decimals,
    )?;

    let final_payer_token_balance =
        accessor::amount(&ctx.accounts.payer_token_ta.to_account_info())?;
    let final_vault_token_balance = accessor::amount(&ctx.accounts.token_vault.to_account_info())?;

    require!(
        final_payer_token_balance == initial_payer_token_balance - amount,
        KaminoVaultError::RewardTopupAmountNotExpected
    );

    require!(
        final_vault_token_balance == initial_vault_token_balance + amount,
        KaminoVaultError::RewardTopupAmountNotExpected
    );

    msg!(
        "Rewards topped up: {} (available: {}, rps: {})",
        amount,
        vault.reward_info.rewards_available,
        vault.reward_info.reward_per_second
    );

    Ok(())
}

#[derive(Accounts)]
pub struct TopupRewards<'info> {
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = token_mint,
        has_one = token_vault,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_mint,
        token::authority = payer,
    )]
    pub payer_token_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

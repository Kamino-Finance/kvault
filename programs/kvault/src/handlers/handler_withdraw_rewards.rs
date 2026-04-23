use anchor_lang::prelude::*;
use anchor_spl::token_interface::{accessor, Mint, TokenAccount, TokenInterface};

use crate::{
    operations::vault_operations,
    utils::token_ops::tokens::{transfer_to_token_account, VaultTransferAccounts},
    KaminoVaultError, VaultState,
};

pub fn process(ctx: Context<WithdrawRewards>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault_state.load_mut()?;
    let current_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let withdraw_amount = vault_operations::withdraw_rewards(vault, amount, current_ts)?;

    let initial_withdraw_token_account_balance =
        accessor::amount(&ctx.accounts.withdraw_token_account.to_account_info())?;
    let initial_vault_token_balance =
        accessor::amount(&ctx.accounts.token_vault.to_account_info())?;

    if withdraw_amount > 0 {
        transfer_to_token_account(
            &VaultTransferAccounts {
                token_program: ctx.accounts.token_program.to_account_info(),
                token_vault: ctx.accounts.token_vault.to_account_info(),
                token_ata: ctx.accounts.withdraw_token_account.to_account_info(),
                token_mint: ctx.accounts.token_mint.to_account_info(),
                base_vault_authority: ctx.accounts.base_vault_authority.to_account_info(),
                vault_state: ctx.accounts.vault_state.to_account_info(),
            },
            vault.base_vault_authority_bump as u8,
            withdraw_amount,
            vault.token_mint_decimals as u8,
        )?;
    }

    let final_withdraw_token_account_balance =
        accessor::amount(&ctx.accounts.withdraw_token_account.to_account_info())?;
    let final_vault_token_balance = accessor::amount(&ctx.accounts.token_vault.to_account_info())?;

    require!(
        final_withdraw_token_account_balance
            == initial_withdraw_token_account_balance + withdraw_amount,
        KaminoVaultError::RewardWithdrawAmountNotExpected
    );

    require!(
        final_vault_token_balance == initial_vault_token_balance - withdraw_amount,
        KaminoVaultError::RewardWithdrawAmountNotExpected
    );

    msg!(
        "Rewards withdrawn: {} (remaining available: {}, rps: {})",
        withdraw_amount,
        vault.reward_info.rewards_available,
        vault.reward_info.reward_per_second
    );

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawRewards<'info> {
    #[account(mut)]
    pub vault_admin_authority: Signer<'info>,

    #[account(
        mut,
        has_one = vault_admin_authority,
        has_one = token_mint,
        has_one = token_vault,
        has_one = base_vault_authority,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: This authority is stored in the vault state
    pub base_vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        token::mint = token_mint,
    )]
    pub withdraw_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

use anchor_lang::{
    err,
    prelude::{Context, Result},
    solana_program::{account_info::AccountInfo, instruction::AccountMeta},
    Discriminator, InstructionData, Key, ToAccountMetas,
};
use kamino_lending::utils::FatAccountLoader;

use crate::{
    handlers::{Invest, WithdrawFromAvailable, WithdrawFromInvested},
    utils::{consts::BASE_VAULT_AUTHORITY_SEED, cpi_mem::CpiMemoryLender},
    KaminoVaultError, WithdrawPendingFees, MAX_RESERVES,
};

pub fn cpi_refresh_reserves<'a, 'info>(
    cpi: &mut CpiMemoryLender,
    reserve_account_infos_iter: impl Iterator<Item = &'a AccountInfo<'info>>,
    reserve_count: usize,
) -> Result<()>
where
    'info: 'a,
{
    if reserve_count == 0 {
        return Ok(());
    }
    let mut accounts_metadatas = [(); MAX_RESERVES * 2].map(|_| AccountMeta::default());
    let mut num_reserves = 0_usize;
    for (account_meta, reserve_account_info) in accounts_metadatas
        .chunks_mut(2)
        .zip(reserve_account_infos_iter)
    {
        account_meta[0] = AccountMeta::new(*reserve_account_info.key, false);
        // Unchecked is safe because load performs discriminator check.
        let lending_market_pk = FatAccountLoader::<kamino_lending::Reserve>::try_from_unchecked(
            &kamino_lending::id(),
            reserve_account_info,
        )?
        .load()?
        .lending_market;
        account_meta[1] = AccountMeta::new_readonly(lending_market_pk, false);
        num_reserves += 1;
    }

    if reserve_count != num_reserves {
        return err!(KaminoVaultError::MissingReserveForBatchRefresh);
    }

    cpi.program_invoke(
        &kamino_lending::id(),
        &accounts_metadatas[..num_reserves * 2],
        &kamino_lending::instruction::RefreshReservesBatch {
            skip_price_updates: true,
        }
        .data(),
    )
    .map_err(Into::into)
}

pub fn cpi_deposit_reserve_liquidity(
    ctx: &Context<Invest>,
    cpi: &mut CpiMemoryLender,
    base_vault_authority_bump: u8,
    liquidity_amount: u64,
) -> Result<()> {
    let accs = kamino_lending::accounts::DepositReserveLiquidity {
        owner: ctx.accounts.base_vault_authority.key(),
        reserve: ctx.accounts.reserve.key(),
        lending_market: ctx.accounts.lending_market.key(),
        lending_market_authority: ctx.accounts.lending_market_authority.key(),
        reserve_liquidity_mint: ctx.accounts.token_mint.key(),
        reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.key(),
        reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.key(),
        user_source_liquidity: ctx.accounts.token_vault.key(),
        user_destination_collateral: ctx.accounts.ctoken_vault.key(),
        collateral_token_program: ctx.accounts.reserve_collateral_token_program.key(),
        liquidity_token_program: ctx.accounts.token_program.key(),
        instruction_sysvar_account: ctx.accounts.instruction_sysvar_account.key(),
    }
    .to_account_metas(None);

    let mut data = [0_u8; 40];
    data[0..8]
        .copy_from_slice(&kamino_lending::instruction::DepositReserveLiquidity::DISCRIMINATOR);
    let mut writer = &mut data[8..40];
    borsh::to_writer(&mut writer, &liquidity_amount).unwrap();

    let base_vault_authority_bump = vec![base_vault_authority_bump];
    let vault_state_key = ctx.accounts.vault_state.key();
    let inner_seeds = [
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        base_vault_authority_bump.as_ref(),
    ];
    let signer_seeds = &[&inner_seeds[..]];

    cpi.program_invoke_signed(
        &ctx.accounts.klend_program.key(),
        &accs,
        &data,
        signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_redeem_reserve_liquidity_from_withdraw(
    from_available_ctx: &WithdrawFromAvailable,
    from_invested_ctx: &WithdrawFromInvested,
    cpi: &mut CpiMemoryLender,
    base_vault_authority_bump: u8,
    collateral_amount: u64,
) -> Result<()> {
    let from_available_accounts = from_available_ctx;
    let from_invested_accounts = from_invested_ctx;
    let accs = kamino_lending::accounts::RedeemReserveCollateral {
        owner: from_available_accounts.base_vault_authority.key(),
        lending_market: from_invested_accounts.lending_market.key(),
        reserve: from_invested_accounts.reserve.key(),
        lending_market_authority: from_invested_accounts.lending_market_authority.key(),
        reserve_liquidity_mint: from_available_accounts.token_mint.key(),
        reserve_collateral_mint: from_invested_accounts.reserve_collateral_mint.key(),
        reserve_liquidity_supply: from_invested_accounts.reserve_liquidity_supply.key(),
        user_source_collateral: from_invested_accounts.ctoken_vault.key(),
        user_destination_liquidity: from_available_accounts.token_vault.key(),
        collateral_token_program: from_invested_accounts
            .reserve_collateral_token_program
            .key(),
        liquidity_token_program: from_available_accounts.token_program.key(),
        instruction_sysvar_account: from_invested_accounts.instruction_sysvar_account.key(),
    }
    .to_account_metas(None);

    let mut data = [0_u8; 40];
    data[0..8]
        .copy_from_slice(&kamino_lending::instruction::RedeemReserveCollateral::DISCRIMINATOR);
    let mut writer = &mut data[8..40];
    borsh::to_writer(&mut writer, &collateral_amount).unwrap();

    let base_vault_authority_bump = vec![base_vault_authority_bump];
    let vault_state_key = from_available_accounts.vault_state.key();
    let inner_seeds = [
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        base_vault_authority_bump.as_ref(),
    ];
    let signer_seeds = &[&inner_seeds[..]];

    cpi.program_invoke_signed(
        &from_available_accounts.klend_program.key(),
        &accs,
        &data,
        signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_redeem_reserve_liquidity_from_withdraw_pending_fees(
    ctx: &Context<WithdrawPendingFees>,
    cpi: &mut CpiMemoryLender,
    base_vault_authority_bump: u8,
    collateral_amount: u64,
) -> Result<()> {
    let accs = kamino_lending::accounts::RedeemReserveCollateral {
        owner: ctx.accounts.base_vault_authority.key(),
        lending_market: ctx.accounts.lending_market.key(),
        reserve: ctx.accounts.reserve.key(),
        lending_market_authority: ctx.accounts.lending_market_authority.key(),
        reserve_liquidity_mint: ctx.accounts.token_mint.key(),
        reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.key(),
        reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.key(),
        user_source_collateral: ctx.accounts.ctoken_vault.key(),
        user_destination_liquidity: ctx.accounts.token_vault.key(),
        collateral_token_program: ctx.accounts.reserve_collateral_token_program.key(),
        liquidity_token_program: ctx.accounts.token_program.key(),
        instruction_sysvar_account: ctx.accounts.instruction_sysvar_account.key(),
    }
    .to_account_metas(None);

    let mut data = [0_u8; 40];
    data[0..8]
        .copy_from_slice(&kamino_lending::instruction::RedeemReserveCollateral::DISCRIMINATOR);
    let mut writer = &mut data[8..40];
    borsh::to_writer(&mut writer, &collateral_amount).unwrap();

    let base_vault_authority_bump = vec![base_vault_authority_bump];
    let vault_state_key = ctx.accounts.vault_state.key();
    let inner_seeds = [
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        base_vault_authority_bump.as_ref(),
    ];
    let signer_seeds = &[&inner_seeds[..]];

    cpi.program_invoke_signed(
        &ctx.accounts.klend_program.key(),
        &accs,
        &data,
        signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_redeem_reserve_liquidity_from_invest(
    ctx: &Context<Invest>,
    cpi: &mut CpiMemoryLender,
    base_vault_authority_bump: u8,
    collateral_amount: u64,
) -> Result<()> {
    let accs = kamino_lending::accounts::RedeemReserveCollateral {
        owner: ctx.accounts.base_vault_authority.key(),
        lending_market: ctx.accounts.lending_market.key(),
        reserve: ctx.accounts.reserve.key(),
        lending_market_authority: ctx.accounts.lending_market_authority.key(),
        reserve_liquidity_mint: ctx.accounts.token_mint.key(),
        reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.key(),
        reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.key(),
        user_source_collateral: ctx.accounts.ctoken_vault.key(),
        user_destination_liquidity: ctx.accounts.token_vault.key(),
        collateral_token_program: ctx.accounts.reserve_collateral_token_program.key(),
        liquidity_token_program: ctx.accounts.token_program.key(),
        instruction_sysvar_account: ctx.accounts.instruction_sysvar_account.key(),
    }
    .to_account_metas(None);

    let mut data = [0_u8; 40];
    data[0..8]
        .copy_from_slice(&kamino_lending::instruction::RedeemReserveCollateral::DISCRIMINATOR);
    let mut writer = &mut data[8..40];
    borsh::to_writer(&mut writer, &collateral_amount).unwrap();

    let base_vault_authority_bump = vec![base_vault_authority_bump];
    let vault_state_key = ctx.accounts.vault_state.key();
    let inner_seeds = [
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        base_vault_authority_bump.as_ref(),
    ];
    let signer_seeds = &[&inner_seeds[..]];

    cpi.program_invoke_signed(
        &ctx.accounts.klend_program.key(),
        &accs,
        &data,
        signer_seeds,
    )
    .map_err(Into::into)
}

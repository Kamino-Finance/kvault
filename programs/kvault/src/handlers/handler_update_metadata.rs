use anchor_lang::prelude::*;
use anchor_spl::metadata::{Metadata, MetadataAccount};

use crate::{utils::metadata, VaultState};

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateSharesMetadata<'info>>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let vault = &ctx.accounts.vault_state.load()?;

    msg!("name={}, symbol={}, uri={}", name, symbol, uri);
    metadata::update(
        ctx.accounts.vault_state.to_account_info(),
        ctx.accounts.metadata_program.to_account_info(),
        ctx.accounts.base_vault_authority.to_account_info(),
        ctx.accounts.shares_metadata.to_account_info(),
        vault.base_vault_authority_bump,
        metadata::TokenMetadata { name, symbol, uri },
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateSharesMetadata<'info> {
    #[account(mut)]
    pub vault_admin_authority: Signer<'info>,

    #[account(
        has_one = vault_admin_authority,
        has_one = base_vault_authority,
    )]
    pub vault_state: AccountLoader<'info, VaultState>,

    /// CHECK: vault checks this
    pub base_vault_authority: AccountInfo<'info>,

    /// CHECK: validated by the downstream metaplex metadata program
    #[account(
        mut,
        constraint = shares_metadata.update_authority == vault_state.load()?.base_vault_authority,
        constraint = shares_metadata.mint == vault_state.load()?.shares_mint
    )]
    pub shares_metadata: Account<'info, MetadataAccount>,

    pub metadata_program: Program<'info, Metadata>,
}

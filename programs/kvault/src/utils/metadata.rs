use anchor_lang::{
    prelude::{AccountInfo, CpiContext},
    Key, Result, ToAccountInfo,
};
use anchor_spl::metadata::mpl_token_metadata::types::DataV2;

use super::consts::BASE_VAULT_AUTHORITY_SEED;
use crate::gen_signer_seeds;

pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[allow(clippy::too_many_arguments)]
pub fn init<'info>(
    vault_state: AccountInfo<'info>,
    metadata_program: AccountInfo<'info>,
    shares_mint: AccountInfo<'info>,
    shares_mint_authority: AccountInfo<'info>,
    shares_metadata: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    rent: AccountInfo<'info>,
    mint_authority_bump: u64,
    TokenMetadata { name, symbol, uri }: TokenMetadata,
) -> Result<()> {
    let vault_state_key = vault_state.key();
    let seeds = gen_signer_seeds!(
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        mint_authority_bump as u8
    );
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    anchor_spl::metadata::create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            metadata_program.to_account_info(),
            anchor_spl::metadata::CreateMetadataAccountsV3 {
                metadata: shares_metadata,
                mint: shares_mint,
                mint_authority: shares_mint_authority.clone(),
                payer,
                update_authority: shares_mint_authority,
                system_program,
                rent,
            },
            signer_seeds,
        ),
        DataV2 {
            name,
            symbol,
            uri,
            creators: None,
            collection: None,
            seller_fee_basis_points: 0,
            uses: None,
        },
        true,
        true,
        None,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update<'info>(
    vault_state: AccountInfo<'info>,
    metadata_program: AccountInfo<'info>,
    shares_mint_authority: AccountInfo<'info>,
    shares_metadata: AccountInfo<'info>,
    mint_authority_bump: u64,
    TokenMetadata { name, symbol, uri }: TokenMetadata,
) -> Result<()> {
    let vault_state_key = vault_state.key();
    let seeds = gen_signer_seeds!(
        BASE_VAULT_AUTHORITY_SEED,
        vault_state_key.as_ref(),
        mint_authority_bump as u8
    );
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    anchor_spl::metadata::update_metadata_accounts_v2(
        CpiContext::new_with_signer(
            metadata_program,
            anchor_spl::metadata::UpdateMetadataAccountsV2 {
                metadata: shares_metadata,
                update_authority: shares_mint_authority,
            },
            signer_seeds,
        ),
        None,
        Some(DataV2 {
            name,
            symbol,
            uri,
            creators: None,
            collection: None,
            seller_fee_basis_points: 0,
            uses: None,
        }),
        None,
        None,
    )?;

    Ok(())
}

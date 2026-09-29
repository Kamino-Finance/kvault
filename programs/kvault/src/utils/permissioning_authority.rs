use anchor_lang::{error::ErrorCode, prelude::*};

use crate::{KaminoVaultError, VaultState};

pub fn check_permissioning_authority_and_strip<'a, 'info>(
    vault: &VaultState,
    remaining_accounts: &'a [AccountInfo<'info>],
) -> Result<&'a [AccountInfo<'info>]> {
    if vault.permissioning_authority == Pubkey::default() {
        return Ok(remaining_accounts);
    }

    let permissioning_authority = remaining_accounts
        .last()
        .ok_or_else(|| error!(KaminoVaultError::InvalidPermissioningAuthority))?;
    require_keys_eq!(
        permissioning_authority.key(),
        vault.permissioning_authority,
        KaminoVaultError::InvalidPermissioningAuthority
    );
    require!(
        permissioning_authority.is_signer,
        ErrorCode::AccountNotSigner
    );

    Ok(&remaining_accounts[..remaining_accounts.len() - 1])
}

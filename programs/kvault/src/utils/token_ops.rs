use anchor_lang::prelude::{AccountInfo, CpiContext, Result};
use anchor_spl::token_interface::{self};

use super::consts::BASE_VAULT_AUTHORITY_SEED;
use crate::gen_signer_seeds;

pub mod shares {

    use super::*;

    pub fn mint<'info>(
        token_program: AccountInfo<'info>,
        shares_mint: AccountInfo<'info>,
        vault_state: AccountInfo<'info>,
        base_vault_authority: AccountInfo<'info>,
        user_shares_ata: AccountInfo<'info>,
        base_vault_authority_bump: u64,
        shares_to_mint: u64,
    ) -> Result<()> {
        let signer_seeds = gen_signer_seeds!(
            BASE_VAULT_AUTHORITY_SEED,
            vault_state.key.as_ref(),
            base_vault_authority_bump as u8
        );

        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                token_program,
                anchor_spl::token::MintTo {
                    mint: shares_mint,
                    to: user_shares_ata,
                    authority: base_vault_authority,
                },
                &[signer_seeds],
            ),
            shares_to_mint,
        )?;

        Ok(())
    }

    pub fn burn<'info>(
        shares_mint: AccountInfo<'info>,
        user_shares_ata: AccountInfo<'info>,
        user: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        shares_to_burn: u64,
    ) -> Result<()> {
        anchor_spl::token::burn(
            CpiContext::new(
                token_program,
                anchor_spl::token::Burn {
                    mint: shares_mint,
                    from: user_shares_ata,
                    authority: user,
                },
            ),
            shares_to_burn,
        )?;

        Ok(())
    }
}

pub mod tokens {
    use super::*;

    pub struct VaultTransferAccounts<'info> {
        pub token_program: AccountInfo<'info>,
        pub token_vault: AccountInfo<'info>,
        pub token_ata: AccountInfo<'info>,
        pub token_mint: AccountInfo<'info>,
        pub base_vault_authority: AccountInfo<'info>,
        pub vault_state: AccountInfo<'info>,
    }

    pub struct UserTransferAccounts<'info> {
        pub token_program: AccountInfo<'info>,
        pub token_vault: AccountInfo<'info>,
        pub token_ata: AccountInfo<'info>,
        pub token_mint: AccountInfo<'info>,
        pub user_authority: AccountInfo<'info>,
    }

    pub fn transfer_to_token_account(
        accounts: &VaultTransferAccounts,
        base_vault_authority_bump: u8,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        anchor_lang::prelude::msg!("Sending back to user {}", amount);

        let signer_seeds = gen_signer_seeds!(
            BASE_VAULT_AUTHORITY_SEED,
            accounts.vault_state.key.as_ref(),
            base_vault_authority_bump
        );

        if amount > 0 {
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    accounts.token_program.clone(),
                    token_interface::TransferChecked {
                        to: accounts.token_ata.clone(),
                        from: accounts.token_vault.clone(),
                        authority: accounts.base_vault_authority.clone(),
                        mint: accounts.token_mint.clone(),
                    },
                    &[signer_seeds],
                ),
                amount,
                decimals,
            )?;
        }

        Ok(())
    }

    pub fn transfer_to_vault(
        accounts: &UserTransferAccounts,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        token_interface::transfer_checked(
            CpiContext::new(
                accounts.token_program.clone(),
                token_interface::TransferChecked {
                    from: accounts.token_ata.clone(),
                    to: accounts.token_vault.clone(),
                    authority: accounts.user_authority.clone(),
                    mint: accounts.token_mint.clone(),
                },
            ),
            amount,
            decimals,
        )
    }
}

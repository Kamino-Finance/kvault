use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{discriminators, pda, util::*, KVAULT_PROGRAM_ID};

// ---------------------------------------------------------------------------
// redeem_in_kind
// ---------------------------------------------------------------------------

/// Account addresses required by the `redeem_in_kind` instruction.
pub struct RedeemInKindAccounts {
    pub user: Pubkey,
    pub vault_state: Pubkey,
    pub global_config: Pubkey,
    pub base_vault_authority: Pubkey,
    pub reserve: Pubkey,
    pub ctoken_vault: Pubkey,
    pub user_ctoken_ta: Pubkey,
    pub ctoken_mint: Pubkey,
    pub user_shares_ta: Pubkey,
    pub shares_mint: Pubkey,
    pub reserve_collateral_token_program: Pubkey,
    pub shares_token_program: Pubkey,
    pub klend_program: Pubkey,
}

/// Build a raw `redeem_in_kind` instruction.
///
/// Burns `shares_amount` vault shares and transfers the proportional
/// cTokens directly to the user instead of redeeming them for the
/// underlying token. Pass `u64::MAX` to burn the caller's entire share
/// balance — the program clamps `shares_amount` to the balance actually held.
/// `remaining_accounts` must contain the refresh list: writable reserve metas
/// followed by readonly lending-market metas, in allocation-slot order.
///
/// # Farms
///
/// If the vault has a share farm (`VaultState::vault_farm` is set), the
/// caller's shares are staked in the farm and must be unstaked from it (a
/// separate Kamino Farms instruction, prepended to the transaction) before
/// they can be burned here.
pub fn redeem_in_kind(
    accounts: RedeemInKindAccounts,
    shares_amount: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct Args {
        shares_amount: u64,
    }

    let args = Args { shares_amount };
    let mut data = discriminators::REDEEM_IN_KIND.to_vec();
    args.serialize(&mut data).unwrap();

    let (evt_auth, _) = pda::event_authority(&KVAULT_PROGRAM_ID);
    let mut account_metas = vec![
        signer(accounts.user),
        writable(accounts.vault_state),
        readonly(accounts.global_config),
        readonly(accounts.base_vault_authority),
        writable(accounts.reserve),
        writable(accounts.ctoken_vault),
        writable(accounts.user_ctoken_ta),
        writable(accounts.ctoken_mint),
        writable(accounts.user_shares_ta),
        writable(accounts.shares_mint),
        readonly(accounts.reserve_collateral_token_program),
        readonly(accounts.shares_token_program),
        readonly(accounts.klend_program),
        // event_cpi accounts
        readonly(evt_auth),
        readonly(KVAULT_PROGRAM_ID),
    ];
    account_metas.extend(remaining_accounts);

    Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts: account_metas,
        data,
    }
}

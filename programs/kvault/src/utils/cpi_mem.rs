use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        program,
        pubkey::Pubkey,
    },
};

/// Memory manager for CPI calls
///
/// The manager hold memory vectors to hold the accounts and data for CPI calls.
/// It allows to reuse the memory vectors to avoid reallocating memory for each CPI call.
pub struct CpiMemoryLender<'info> {
    /// Memory vectors for accounts
    accounts: Option<Vec<AccountMeta>>,
    /// Memory vectors for data
    data: Option<Vec<u8>>,
    /// Account infos
    accounts_infos: Vec<AccountInfo<'info>>,
}

impl<'info> CpiMemoryLender<'info> {
    /// Create a new memory manager
    pub fn new(
        accounts_infos: Vec<AccountInfo<'info>>,
        max_accounts: usize,
        max_data: usize,
    ) -> Self {
        Self {
            accounts: Some(Vec::with_capacity(max_accounts)),
            data: Some(Vec::with_capacity(max_data)),
            accounts_infos,
        }
    }

    /// Build a memory manager from all the accounts received in an instruction
    pub fn build_cpi_memory_lender(
        mut ctx_accounts: Vec<AccountInfo<'info>>,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Self {
        ctx_accounts.extend_from_slice(remaining_accounts);
        CpiMemoryLender::new(ctx_accounts, 64, 128)
    }

    /// Create an instruction
    fn ix(
        &mut self,
        program_id: &Pubkey,
        ix_accounts: &[AccountMeta],
        ix_data: &[u8],
    ) -> Instruction {
        let mut accounts = self.accounts.take().unwrap();
        let mut data = self.data.take().unwrap();
        accounts.clear();
        data.clear();
        accounts.extend_from_slice(ix_accounts);
        data.extend_from_slice(ix_data);
        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Return the accounts and data vectors
    fn del_ix(&mut self, ix: Instruction) {
        let Instruction {
            program_id: _,
            accounts: ix_accounts,
            data: ix_data,
        } = ix;
        self.accounts = Some(ix_accounts);
        self.data = Some(ix_data);
    }

    pub fn program_invoke(
        &mut self,
        program_id: &Pubkey,
        ix_accounts: &[AccountMeta],
        ix_data: &[u8],
    ) -> ProgramResult {
        let ix = self.ix(program_id, ix_accounts, ix_data);
        let res = program::invoke(&ix, &self.accounts_infos);
        self.del_ix(ix);
        res
    }

    pub fn program_invoke_signed(
        &mut self,
        program_id: &Pubkey,
        ix_accounts: &[AccountMeta],
        ix_data: &[u8],
        signer_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let ix = self.ix(program_id, ix_accounts, ix_data);
        let res = program::invoke_signed(&ix, &self.accounts_infos, signer_seeds);
        self.del_ix(ix);
        res
    }
}

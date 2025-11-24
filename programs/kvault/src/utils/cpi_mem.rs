use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
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
        self.program_invoke_signed(program_id, ix_accounts, ix_data, &[])
    }

    pub fn program_invoke_signed(
        &mut self,
        program_id: &Pubkey,
        ix_accounts: &[AccountMeta],
        ix_data: &[u8],
        signer_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let ix = self.ix(program_id, ix_accounts, ix_data);
        let (res, ix) = invoke_signed_and_recover_ix(ix, &self.accounts_infos, signer_seeds);
        self.del_ix(ix);
        res
    }
}

/// Mimics the original [solana_program::program::invoke_signed()] with one important distinction:
/// instead of borrowing an [Instruction] instance, it consumes one and then returns the equivalent
/// one (alongside the original result).
///
/// # Why?
///
/// In a Rust-native setting (non-SBF) - it changes nothing (and is only mildly cumbersome).
///
/// However, in an SBF setting, it allows us to bypass the memory inefficiency living deep within
/// the `solana-program` library, looking like:
///
/// `let instruction = StableInstruction::from(instruction.clone());`
///
/// The `.clone()` above destroys all our efforts of avoiding heap allocation (both for the
/// instruction data *and* accounts).
///
/// # How?
///
/// The implementation below is 90% of [solana_program::program::invoke_signed_unchecked()], with
/// removed `.clone()` an added "steal back the `Vec`s" logic at the end.
fn invoke_signed_and_recover_ix(
    instruction: Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> (ProgramResult, Instruction) {
    #[cfg(target_os = "solana")]
    {
        let stable_instruction =
            solana_program::stable_layout::stable_instruction::StableInstruction::from(instruction); // Our change: the original does `instruction.clone()`
        let numeric_result = unsafe {
            solana_program::syscalls::sol_invoke_signed_rust(
                &stable_instruction as *const _ as *const u8,
                account_infos as *const _ as *const u8,
                account_infos.len() as u64,
                signers_seeds as *const _ as *const u8,
                signers_seeds.len() as u64,
            )
        };
        let result = match numeric_result {
            solana_program::entrypoint::SUCCESS => Ok(()),
            numeric_error => Err(ProgramError::from(numeric_error)),
        };

        // Our change: we recover the `Instruction` instance from the parts of `StableInstruction`.
        let instruction = Instruction {
            program_id: stable_instruction.program_id,
            accounts: Vec::from(stable_instruction.accounts), // This is not copying; this is actually a custom `From<StableVec>`...
            data: Vec::from(stable_instruction.data), // ... which steals back the pointer, like `Vec::from_raw_parts()`.
        };
        (result, instruction) // Our change: we have to return this `Instruction` (since the original was only borrowing, and we are consuming)
    }

    #[cfg(not(target_os = "solana"))]
    {
        let result =
            solana_program::program::invoke_signed(&instruction, account_infos, signers_seeds);
        (result, instruction) // Our change: the non-SBF variant is exactly the same, except we have to return the consumed instruction
    }
}

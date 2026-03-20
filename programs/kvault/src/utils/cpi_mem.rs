use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
};





pub struct CpiMemoryLender<'info> {

    accounts: Option<Vec<AccountMeta>>,

    data: Option<Vec<u8>>,

    accounts_infos: Vec<AccountInfo<'info>>,
}

impl<'info> CpiMemoryLender<'info> {

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


    pub fn build_cpi_memory_lender(
        mut ctx_accounts: Vec<AccountInfo<'info>>,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Self {
        ctx_accounts.extend_from_slice(remaining_accounts);
        CpiMemoryLender::new(ctx_accounts, 64, 128)
    }


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





















fn invoke_signed_and_recover_ix(
    instruction: Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> (ProgramResult, Instruction) {
    #[cfg(target_os = "solana")]
    {
        let stable_instruction =
            solana_program::stable_layout::stable_instruction::StableInstruction::from(instruction);
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

       
        let instruction = Instruction {
            program_id: stable_instruction.program_id,
            accounts: Vec::from(stable_instruction.accounts),
            data: Vec::from(stable_instruction.data),
        };
        (result, instruction)
    }

    #[cfg(not(target_os = "solana"))]
    {
        let result =
            solana_program::program::invoke_signed(&instruction, account_infos, signers_seeds);
        (result, instruction)
    }
}

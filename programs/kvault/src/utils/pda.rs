use anchor_lang::prelude::Pubkey;

use crate::utils::consts::GLOBAL_CONFIG_STATE_SEEDS;

pub fn program_data() -> Pubkey {
    program_data_program_id(&crate::ID)
}

pub fn program_data_program_id(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::ID,
    )
    .0
}

pub fn global_config() -> Pubkey {
    global_config_program_id(&crate::ID)
}

pub fn global_config_program_id(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[GLOBAL_CONFIG_STATE_SEEDS], program_id).0
}

use interchain_token_transfer_gmp::InterchainTransfer;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::state::RootPDA;

impl Processor {
    /// Processes an instruction.
    pub fn interchain_transfer(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        _input: InterchainTransfer,
        _root_pda: &RootPDA,
    ) -> ProgramResult {
        todo!()
    }
}

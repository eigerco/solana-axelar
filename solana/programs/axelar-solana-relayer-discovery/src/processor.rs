//! Processor for the Solana gas service

use borsh::BorshDeserialize;
use set_instruction::process_set_instruction;
use get_instruction::process_get_instruction;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};


use crate::check_program_account;
use crate::instructions::RelayerDiscoveryInstruction;

mod set_instruction;
mod get_instruction;

/// Processes an instruction.
///
/// # Errors
/// - if the ix processing resulted in an error
#[allow(clippy::todo)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    let instruction = RelayerDiscoveryInstruction::try_from_slice(input)?;
    check_program_account(*program_id)?;

    match instruction {
        RelayerDiscoveryInstruction::SetInstruction { execution_info, destination_address} => {
            process_set_instruction(program_id, accounts, execution_info, destination_address)
        }
        RelayerDiscoveryInstruction::GetInstruction{ destination_address } => {
            process_get_instruction(program_id, accounts, destination_address)
        }
    }
}
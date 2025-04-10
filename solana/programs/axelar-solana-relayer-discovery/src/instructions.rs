//! # Instruction Module
//!
//! This module provides constructors and definitions for all instructions that can be issued to the

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::system_program;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::state::RelayerExecutionInfo;
use crate::{get_relayer_execution_pda, get_singing_pda, ID};

/// Top-level instructions supported by the Axelar Solana Gas Service program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum RelayerDiscoveryInstruction {
    /// Set an instruction
    SetInstruction {
        /// The instruction to run for the program.
        execution_info: RelayerExecutionInfo,
        /// The program id.
        destination_address: Pubkey,
    },

    /// Get an instruction
    GetInstruction {
        /// The program id.
        destination_address: Pubkey,
    }
}

/// Builds an instruction to set an instruction.
///
/// # Errors
/// - ix data cannot be serialized
pub fn set_instruction(
    payer: &Pubkey,
    destination_address: Pubkey,
    execution_info: RelayerExecutionInfo,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&RelayerDiscoveryInstruction::SetInstruction { execution_info, destination_address })?;
    
    let program_id = ID;
    let (signing_pda_address, _) = get_singing_pda(&destination_address);
    let (relayer_execution_pda_address, _) = get_relayer_execution_pda(&program_id, &destination_address);

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(relayer_execution_pda_address, false),
        AccountMeta::new_readonly(signing_pda_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to get the relayer instruction
///
/// # Errors
/// - ix data cannot be serialized
pub fn get_instruction(
    destination_address: Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&RelayerDiscoveryInstruction::GetInstruction { destination_address})?;
    let program_id = ID;
    let (relayer_execution_pda_address, _) = get_relayer_execution_pda(&program_id, &destination_address);

    let accounts = vec![
        AccountMeta::new_readonly(relayer_execution_pda_address, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data: ix_data,
    })
}
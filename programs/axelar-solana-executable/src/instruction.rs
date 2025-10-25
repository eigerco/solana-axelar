//! Instruction module for the Axelar Memo program.

use anchor_discriminators_macros::InstructionDiscriminator;
use borsh::to_vec;
use relayer_discovery::find_transaction_pda;
pub use solana_program;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Instructions supported by the Axelar Memo program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, InstructionDiscriminator)]
pub enum AxelarExecutableInstruction {
    /// Initialize the transaction_pda.
    Initialize,
    /// Get the execute transaction after calculating the accounts required.
    GetTransaction {
        /// The payload from the gateway.
        payload: Vec<u8>
    }
}

/// Creates a [`AxelarMemoInstruction::Initialize`] instruction.
pub fn initialize(payer: &Pubkey) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarExecutableInstruction::Initialize)?;
    
    let (transaction_pda, _) = find_transaction_pda(&crate::ID);

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(transaction_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Get the final transaction to execute.
pub fn get_transaction(payload: Vec<u8>) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarExecutableInstruction::GetTransaction { payload })?;

    let accounts = vec![];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    }) 
}

//! Program state processor

use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::executable::{
    validate_message, AxelarMessagePayload, PROGRAM_ACCOUNTS_START_INDEX,
};
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use axelar_solana_its::executable::{
    AxelarInterchainTokenExecuteInfo, MaybeAxelarInterchainTokenExecutablePayload,
};
use borsh::{self, to_vec, BorshDeserialize};
use mpl_token_metadata::accounts::Metadata;
use program_utils::{check_program_account, pda::ValidPDA};
use relayer_discovery::find_transaction_pda;
use relayer_discovery::structs::{RelayerAccount, RelayerData, RelayerInstruction, RelayerTransaction};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};
use std::str::from_utf8;

use crate::{assert_storage_pda_seeds, get_storage_pda};
use crate::instruction::AxelarExecutableInstruction;
use crate::state::Payload;

/// Instruction processor
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id, crate::check_id)?;
    
    let instruction = AxelarExecutableInstruction::try_from_slice(input)?;

    match instruction {
        AxelarExecutableInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize_relayer_discovery_transaction_pda(program_id, accounts)?;
        }
        AxelarExecutableInstruction::GetTransaction { payload } => {
            msg!("Instruction: GetTransaction1");
            process_get_transaction(program_id, payload)?;
        }
    }

    Ok(())
}
/// This function is used to initialize the program.
pub fn process_initialize_relayer_discovery_transaction_pda(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let transaction_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: Memo counter PDA Account is not initialized
    transaction_pda.check_uninitialized_pda()?;

    let (found_pda, bump) = find_transaction_pda(program_id);
    // Check: counter PDA account uses the canonical bump.
    if &found_pda != transaction_pda.key {
        Err(ProgramError::InvalidInstructionData)?
    };

    let transaction = RelayerTransaction::Discovery(RelayerInstruction {
        program_id: crate::ID,
        accounts: vec![],
        data: vec![
            RelayerData::Bytes(to_vec(&AxelarExecutableInstruction::GetTransaction { payload: vec![] }).unwrap()[0..8].to_vec()),
            RelayerData::Payload,
        ],
    });

    transaction.init(&crate::ID, system_account, payer, transaction_pda, &[&relayer_discovery::TRANSACTION_PDA_SEED, &[bump]])
}

/// This function is used to initialize the program.
pub fn process_get_transaction(
    program_id: &Pubkey,
    payload: Vec<u8>,
) -> Result<RelayerTransaction, ProgramError> {
    let payload = Payload::deserialize(&mut payload.as_slice())?;
    
    let (storage_pda, _) = get_storage_pda(payload.storage_id);
    let system_program = system_program::id();

    let transaction = RelayerTransaction::Discovery(RelayerInstruction {
        program_id: crate::ID,
        accounts: vec![
            RelayerAccount::Payer(1000),
            RelayerAccount::IncomingMessage,
            RelayerAccount::Account { pubkey: storage_pda, is_writable: true },
            RelayerAccount::Account { pubkey: system_program, is_writable: false },
        ],
        data: vec![
            RelayerData::Bytes(vec![2]),
            RelayerData::Payload,
            RelayerData::Message,
        ],
    });

    Ok(transaction)
}

pub fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    message: Message,
    payload: Vec<u8>,
) -> Result<(), ProgramError> {
    Ok(())
}

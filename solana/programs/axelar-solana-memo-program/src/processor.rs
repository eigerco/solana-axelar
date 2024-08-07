//! Program state processor

use std::str::from_utf8;

use axelar_executable::{validate_message, AxelarCallableInstruction, AxelarExecutablePayload};
use borsh::BorshDeserialize;
use program_utils::{check_program_account, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::assert_counter_pda_seeds;
use crate::instruction::AxelarMemoInstruction;
use crate::state::Counter;
/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id, crate::check_id)?;

    let payload = AxelarCallableInstruction::<AxelarMemoInstruction>::try_from_slice(input)?;
    match payload {
        AxelarCallableInstruction::AxelarExecute(payload) => {
            msg!("Instruction: AxelarExecute");
            process_message_from_axelar(program_id, accounts, payload)
        }
        AxelarCallableInstruction::Native(payload) => {
            msg!("Instruction: Native");
            process_native_ix(program_id, accounts, payload)
        }
    }
}

/// Process a message submitted by the relayer which originates from the Axelar
/// network
pub fn process_message_from_axelar(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    payload: AxelarExecutablePayload,
) -> ProgramResult {
    validate_message(program_id, accounts, &payload)?;

    let account_info_iter = &mut accounts.iter();
    let _gateway_approved_message_pda = next_account_info(account_info_iter)?;
    let _signing_pda = next_account_info(account_info_iter)?;
    let _gateway_root_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;
    let counter_pda = next_account_info(account_info_iter)?;

    // Iterate over the rest of the provided accounts
    for account_info in account_info_iter {
        // NOTE: The accounts WILL NEVER be signers, but they MAY be writable
        msg!(
            "Provided account {:?}-{}-{}",
            account_info.key,
            account_info.is_signer,
            account_info.is_writable
        );
    }

    let memo = from_utf8(&payload.payload_without_accounts).map_err(|err| {
        msg!("Invalid UTF-8, from byte {}", err.valid_up_to());
        ProgramError::InvalidInstructionData
    })?;
    msg!("Memo (len {}): {:?}", memo.len(), memo);

    let mut counter_pda_account = counter_pda.check_initialized_pda::<Counter>(program_id)?;
    counter_pda_account.counter += 1;
    let mut data = counter_pda.try_borrow_mut_data()?;
    counter_pda_account.pack_into_slice(&mut data);

    Ok(())
}

/// Process a native instruction submitted by another program or user ON the
/// Solana network
pub fn process_native_ix(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    payload: AxelarMemoInstruction,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    match payload {
        AxelarMemoInstruction::SendToGateway {
            memo,
            destination_chain,
            destination_address,
        } => {
            msg!("Instruction: SendToGateway");
            let sender = next_account_info(account_info_iter)?;
            let gateway_root_pda = next_account_info(account_info_iter)?;
            invoke(
                &gateway::instructions::call_contract(
                    *gateway_root_pda.key,
                    *sender.key,
                    destination_chain,
                    destination_address,
                    memo.into_bytes(),
                )?,
                &[sender.clone(), gateway_root_pda.clone()],
            )?;
        }
        AxelarMemoInstruction::Initialize { counter_pda_bump } => {
            process_initialize_memo_program_counter(program_id, accounts, counter_pda_bump)?;
        }
    }

    Ok(())
}

/// This function is used to initialize the program.
pub fn process_initialize_memo_program_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    bump: u8,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let counter_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    let counter = crate::state::Counter { counter: 0, bump };

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: Memo counter PDA Account is not initialized
    counter_pda.check_uninitialized_pda()?;
    // Check: counter PDA account uses the canonical bump.
    assert_counter_pda_seeds(&counter, counter_pda.key, gateway_root_pda.key);

    program_utils::init_pda(
        payer,
        counter_pda,
        program_id,
        system_account,
        counter,
        &[gateway_root_pda.key.as_ref(), &[bump]],
    )
}

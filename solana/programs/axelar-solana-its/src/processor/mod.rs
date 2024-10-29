//! Program state processor

use alloy_primitives::U256;
use axelar_executable::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_rkyv_encoding::types::GmpMetadata;
use interchain_token_transfer_gmp::GMPPayload;
use itertools::Itertools;
use program_utils::{check_rkyv_initialized_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::instructions::{
    derive_its_accounts, its_account_indices, Bumps, InterchainTokenServiceInstruction,
    OutboundInstruction,
};
use crate::state::token_manager::TokenManager;
use crate::state::InterchainTokenService;
use crate::{check_program_account, seed_prefixes};

pub mod interchain_token;
pub mod interchain_transfer;
pub mod token_manager;

pub(crate) trait LocalAction {
    fn process_local_action<'a>(
        self,
        payer: &AccountInfo<'a>,
        accounts: &[AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult;
}

impl LocalAction for GMPPayload {
    fn process_local_action<'a>(
        self,
        payer: &AccountInfo<'a>,
        accounts: &[AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult {
        match self {
            Self::InterchainTransfer(inner) => inner.process_local_action(payer, accounts, bumps),
            Self::DeployInterchainToken(inner) => {
                inner.process_local_action(payer, accounts, bumps)
            }
            Self::DeployTokenManager(inner) => inner.process_local_action(payer, accounts, bumps),
        }
    }
}

/// Processes an instruction.
///
/// # Errors
///
/// A `ProgramError` containing the error that occurred is returned. Log
/// messages are also generated with more detailed information.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    instruction_data: &[u8],
) -> ProgramResult {
    check_program_account(*program_id)?;
    let instruction = match InterchainTokenServiceInstruction::from_bytes(instruction_data) {
        Ok(instruction) => instruction,
        Err(err) => {
            msg!("Failed to deserialize instruction: {:?}", err);
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    match instruction {
        InterchainTokenServiceInstruction::Initialize { pda_bump } => {
            process_initialize(accounts, pda_bump)?;
        }
        InterchainTokenServiceInstruction::ItsGmpPayload {
            abi_payload,
            gmp_metadata,
            bumps,
        } => {
            process_inbound_its_gmp_payload(accounts, gmp_metadata, &abi_payload, bumps)?;
        }
        InterchainTokenServiceInstruction::DeployInterchainToken { params, bumps } => {
            process_its_native(accounts, params, bumps)?;
        }
        InterchainTokenServiceInstruction::DeployTokenManager { params, bumps } => {
            process_its_native(accounts, params, bumps)?;
        }
    }

    Ok(())
}

fn process_initialize(accounts: &[AccountInfo<'_>], pda_bump: u8) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let its_root_pda = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: PDA Account is not initialized
    its_root_pda.check_uninitialized_pda()?;

    // Check the bump seed is correct
    crate::check_initialization_bump(pda_bump, its_root_pda.key, gateway_root_pda.key)?;
    let data = InterchainTokenService::new(pda_bump);

    program_utils::init_rkyv_pda::<{ InterchainTokenService::LEN }, _>(
        payer,
        its_root_pda,
        &crate::id(),
        system_account,
        data,
        &[
            crate::seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[pda_bump],
        ],
    )?;

    Ok(())
}

fn process_inbound_its_gmp_payload(
    accounts: &[AccountInfo<'_>],
    gmp_metadata: GmpMetadata,
    abi_payload: &[u8],
    bumps: Bumps,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    validate_with_gmp_metadata(&crate::id(), gateway_accounts, gmp_metadata, abi_payload)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;

    let payload =
        GMPPayload::decode(abi_payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    validate_its_accounts(instruction_accounts, gateway_root_pda.key, &payload, bumps)?;
    payload.process_local_action(payer, instruction_accounts, bumps)
}

fn process_its_native<T>(
    accounts: &[AccountInfo<'_>],
    mut payload: T,
    bumps: Option<Bumps>,
) -> ProgramResult
where
    T: TryInto<GMPPayload> + OutboundInstruction,
{
    let (payer, other_accounts) = accounts
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let gas_value = payload.gas_value();
    let destination_chain = payload.destination_chain();

    let payload: GMPPayload = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    match (destination_chain, bumps) {
        (Some(chain), _) => {
            process_outbound_its_gmp_payload(other_accounts, &payload, chain, gas_value.into())?;
        }
        (None, Some(bumps)) => {
            let (_gateway_root_pda, other_accounts) = other_accounts
                .split_first()
                .ok_or(ProgramError::Custom(1))?;

            payload.process_local_action(payer, other_accounts, bumps)?;
        }
        (None, None) => {
            msg!("Missing ITS PDA bumps");
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    Ok(())
}

/// Processes an outgoing [`InterchainTransfer`], [`DeployInterchainToken`] or
/// [`DeployTokenManager`].
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
fn process_outbound_its_gmp_payload(
    accounts: &[AccountInfo<'_>],
    payload: &GMPPayload,
    destination_chain: String,
    _gas_value: U256,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let its_root_pda_data = its_root_pda.try_borrow_data()?;
    let its_state = check_rkyv_initialized_pda::<InterchainTokenService>(
        &crate::id(),
        its_root_pda,
        *its_root_pda_data,
    )?;

    // TODO: Get chain's trusted address.
    let destination_address = String::new();

    // TODO: Call gas service to pay gas fee.

    invoke_signed(
        &gateway::instructions::call_contract(
            *gateway_root_pda.key,
            *its_root_pda.key,
            destination_chain,
            destination_address,
            payload.encode(),
        )?,
        &[its_root_pda.clone(), gateway_root_pda.clone()],
        &[&[
            seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[its_state.bump],
        ]],
    )?;

    Ok(())
}

fn validate_its_accounts(
    accounts: &[AccountInfo<'_>],
    gateway_root_pda: &Pubkey,
    payload: &GMPPayload,
    bumps: Bumps,
) -> ProgramResult {
    // In this case we cannot derive the mint account, so we just use what we got
    // and check later against the mint within the `TokenManager` PDA.
    let maybe_mint = if let GMPPayload::InterchainTransfer(_) = payload {
        accounts
            .get(its_account_indices::TOKEN_MINT_INDEX)
            .map(|account| *account.key)
    } else {
        None
    };

    let token_program = accounts
        .get(its_account_indices::TOKEN_PROGRAM_INDEX)
        .map(|account| *account.key)
        .ok_or(ProgramError::InvalidAccountData)?;

    let (derived_its_accounts, new_bumps) = derive_its_accounts(
        gateway_root_pda,
        payload,
        token_program,
        maybe_mint,
        Some(bumps),
    )?;

    if new_bumps != bumps {
        return Err(ProgramError::InvalidAccountData);
    }

    for element in accounts.iter().zip_longest(derived_its_accounts.iter()) {
        match element {
            itertools::EitherOrBoth::Both(provided, derived) => {
                if provided.key != &derived.pubkey {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            itertools::EitherOrBoth::Left(_) | itertools::EitherOrBoth::Right(_) => {
                return Err(ProgramError::InvalidAccountData)
            }
        }
    }

    // Now we validate the mint account passed for `InterchainTransfer`
    if let Some(mint) = maybe_mint {
        let token_manager_pda = accounts
            .get(its_account_indices::TOKEN_MANAGER_PDA_INDEX)
            .ok_or(ProgramError::InvalidAccountData)?;
        let token_manager_pda_data = token_manager_pda.try_borrow_data()?;

        let token_manager = check_rkyv_initialized_pda::<TokenManager>(
            &crate::id(),
            token_manager_pda,
            token_manager_pda_data.as_ref(),
        )?;

        if token_manager.token_address.as_ref() != mint.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    Ok(())
}

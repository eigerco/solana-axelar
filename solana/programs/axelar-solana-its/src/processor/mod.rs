//! Program state processor

use alloy_primitives::U256;
use axelar_executable::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::{BytemuckedPda, GatewayConfig};
use interchain_token_transfer_gmp::{GMPPayload, SendToHub};
use program_utils::{StorableArchive, ValidPDA};
use role_management::processor::{
    ensure_signer_roles, ensure_upgrade_authority, RoleManagementAccounts,
};
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use self::token_manager::SetFlowLimitAccounts;
use crate::instructions::{
    self, InterchainTokenServiceInstruction, OptionalAccountsFlags, OutboundInstructionInputs,
};
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, check_program_account, seed_prefixes, Roles};

pub mod interchain_token;
pub mod interchain_transfer;
pub mod token_manager;

const ITS_HUB_CHAIN_NAME: &str = "axelar";
const ITS_HUB_ROUTING_IDENTIFIER: &str = "hub";

pub(crate) trait LocalAction {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        optional_accounts_flags: OptionalAccountsFlags,
        message: Option<Message>,
    ) -> ProgramResult;
}

impl LocalAction for GMPPayload {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        optional_accounts_flags: OptionalAccountsFlags,
        message: Option<Message>,
    ) -> ProgramResult {
        match self {
            Self::InterchainTransfer(inner) => {
                inner.process_local_action(payer, accounts, optional_accounts_flags, message)
            }
            Self::DeployInterchainToken(inner) => {
                inner.process_local_action(payer, accounts, optional_accounts_flags, message)
            }
            Self::DeployTokenManager(inner) => {
                inner.process_local_action(payer, accounts, optional_accounts_flags, message)
            }
            Self::SendToHub(_) | Self::ReceiveFromHub(_) => {
                msg!("Unsupported local action");
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}

/// Processes an instruction.
///
/// # Errors
///
/// A `ProgramError` containing the error that occurred is returned. Log
/// messages are also generated with more detailed information.
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
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
        InterchainTokenServiceInstruction::Initialize => {
            process_initialize(program_id, accounts)?;
        }
        InterchainTokenServiceInstruction::SetPauseStatus { paused } => {
            process_set_pause_status(accounts, paused)?;
        }
        InterchainTokenServiceInstruction::ItsGmpPayload {
            abi_payload,
            message,
            optional_accounts_flags,
        } => {
            process_inbound_its_gmp_payload(
                accounts,
                message,
                &abi_payload,
                optional_accounts_flags,
            )?;
        }
        InterchainTokenServiceInstruction::DeployInterchainToken { params } => {
            process_its_native_deploy_call(accounts, params, OptionalAccountsFlags::empty())?;
        }
        InterchainTokenServiceInstruction::DeployTokenManager {
            params,
            optional_accounts_mask,
        } => {
            process_its_native_deploy_call(accounts, params, optional_accounts_mask)?;
        }
        InterchainTokenServiceInstruction::InterchainTransfer { params } => {
            interchain_transfer::process_outbound_transfer(params, accounts)?;
        }
        InterchainTokenServiceInstruction::SetFlowLimit { flow_limit } => {
            let mut instruction_accounts = SetFlowLimitAccounts::try_from(accounts)?;

            ensure_signer_roles(
                &crate::id(),
                instruction_accounts.its_root_pda,
                instruction_accounts.flow_limiter,
                instruction_accounts.its_user_roles_pda,
                Roles::OPERATOR,
            )?;

            instruction_accounts.flow_limiter = instruction_accounts.its_root_pda;
            token_manager::set_flow_limit(&instruction_accounts, flow_limit)?;
        }
        InterchainTokenServiceInstruction::OperatorInstruction(operator_instruction) => {
            process_operator_instruction(accounts, operator_instruction)?;
        }
        InterchainTokenServiceInstruction::TokenManagerInstruction(token_manager_instruction) => {
            token_manager::process_instruction(accounts, token_manager_instruction)?;
        }
        InterchainTokenServiceInstruction::InterchainTokenInstruction(
            interchain_token_instruction,
        ) => {
            interchain_token::process_instruction(accounts, interchain_token_instruction)?;
        }
        InterchainTokenServiceInstruction::CallContractWithInterchainToken { params } => {
            if params.data.is_empty() {
                return Err(ProgramError::InvalidInstructionData);
            }

            interchain_transfer::process_outbound_transfer(params, accounts)?;
        }
    }

    Ok(())
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let gateway_root_pda_account = next_account_info(account_info_iter)?;
    let its_root_pda_account = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;
    let operator = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check: PDA Account is not initialized
    its_root_pda_account.check_uninitialized_pda()?;

    // Check: Gateway Root PDA Account is valid.
    let gateway_config_data = gateway_root_pda_account.try_borrow_data()?;
    let gateway_config = GatewayConfig::read(&gateway_config_data)?;
    axelar_solana_gateway::assert_valid_gateway_root_pda(
        gateway_config.bump,
        gateway_root_pda_account.key,
    )?;

    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda(gateway_root_pda_account.key);
    let its_root_config = InterchainTokenService::new(its_root_pda_bump);
    its_root_config.init(
        &crate::id(),
        system_account,
        payer,
        its_root_pda_account,
        &[
            crate::seed_prefixes::ITS_SEED,
            gateway_root_pda_account.key.as_ref(),
            &[its_root_pda_bump],
        ],
    )?;

    let (_user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, operator.key);
    let operator_user_roles = UserRoles::new(Roles::OPERATOR, user_roles_pda_bump);
    let signer_seeds = &[
        role_management::seed_prefixes::USER_ROLES_SEED,
        its_root_pda.as_ref(),
        operator.key.as_ref(),
        &[user_roles_pda_bump],
    ];

    operator_user_roles.init(
        program_id,
        system_account,
        payer,
        user_roles_account,
        signer_seeds,
    )?;

    Ok(())
}

fn process_inbound_its_gmp_payload<'a>(
    accounts: &'a [AccountInfo<'a>],
    message: Message,
    abi_payload: &[u8],
    optional_accounts_flags: OptionalAccountsFlags,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    if message.source_address != ITS_HUB_ROUTING_IDENTIFIER {
        msg!("Untrusted source address: {}", message.source_address);
        return Err(ProgramError::InvalidInstructionData);
    }

    validate_with_gmp_metadata(gateway_accounts, &message, abi_payload)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let its_root_pda_account = next_account_info(accounts_iter)?;

    let its_root_config =
        InterchainTokenService::load_readonly(&crate::id(), its_root_pda_account)?;
    assert_valid_its_root_pda(
        its_root_pda_account,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;

    if its_root_config.paused {
        msg!("The Interchain Token Service is currently paused.");
        return Err(ProgramError::Immutable);
    }

    let GMPPayload::ReceiveFromHub(inner) =
        GMPPayload::decode(abi_payload).map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("Unsupported GMP payload");
        return Err(ProgramError::InvalidInstructionData);
    };

    let payload =
        GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    payload.process_local_action(
        payer,
        instruction_accounts,
        optional_accounts_flags,
        Some(message),
    )
}

fn process_its_native_deploy_call<'a, T>(
    accounts: &'a [AccountInfo<'a>],
    mut payload: T,
    optional_accounts_flags: OptionalAccountsFlags,
) -> ProgramResult
where
    T: TryInto<GMPPayload> + OutboundInstructionInputs,
{
    let (payer, other_accounts) = accounts
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let gas_value = payload.gas_value();
    let destination_chain = payload.destination_chain();

    let payload: GMPPayload = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    match destination_chain {
        Some(chain) => {
            process_outbound_its_gmp_payload(other_accounts, &payload, chain, gas_value.into())?;
        }
        None => {
            payload.process_local_action(payer, other_accounts, optional_accounts_flags, None)?;
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
fn process_outbound_its_gmp_payload<'a>(
    accounts: &'a [AccountInfo<'a>],
    payload: &GMPPayload,
    destination_chain: String,
    _gas_value: U256,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let its_root_config = InterchainTokenService::load_readonly(&crate::id(), its_root_pda)?;
    assert_valid_its_root_pda(its_root_pda, gateway_root_pda.key, its_root_config.bump)?;
    if its_root_config.paused {
        msg!("The Interchain Token Service is currently paused.");
        return Err(ProgramError::Immutable);
    }

    let hub_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        destination_chain,
        payload: payload.encode().into(),
    });

    // TODO: Call gas service to pay gas fee.

    invoke_signed(
        &axelar_solana_gateway::instructions::call_contract(
            axelar_solana_gateway::id(),
            *gateway_root_pda.key,
            *its_root_pda.key,
            ITS_HUB_CHAIN_NAME.to_owned(),
            ITS_HUB_ROUTING_IDENTIFIER.to_owned(),
            hub_payload.encode(),
        )?,
        &[its_root_pda.clone(), gateway_root_pda.clone()],
        &[&[
            seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[its_root_config.bump],
        ]],
    )?;

    Ok(())
}

fn process_operator_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::operator::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;

    let its_config =
        InterchainTokenService::load_readonly(&crate::id(), role_management_accounts.resource)?;
    assert_valid_its_root_pda(
        role_management_accounts.resource,
        gateway_root_pda.key,
        its_config.bump,
    )?;

    match instruction {
        instructions::operator::Instruction::TransferOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }

            role_management::processor::transfer(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::ProposeOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }
            role_management::processor::propose(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::AcceptOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }
            role_management::processor::accept(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::empty(),
            )?;
        }
    }

    Ok(())
}

fn process_set_pause_status(accounts: &[AccountInfo<'_>], paused: bool) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;

    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;
    let mut its_root_config = InterchainTokenService::load(&crate::id(), its_root_pda)?;
    assert_valid_its_root_pda(
        its_root_pda,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;
    its_root_config.paused = paused;
    its_root_config.store(its_root_pda)?;

    Ok(())
}

#![allow(clippy::too_many_arguments)]
//! Program state processor
use axelar_solana_gateway::error::GatewayError;
use axelar_solana_gateway::state::GatewayConfig;
use borsh::BorshDeserialize;
use program_utils::{BorshPda, BytemuckedPda, ValidPDA};
use role_management::processor::{
    ensure_signer_roles, ensure_upgrade_authority, RoleManagementAccounts,
};
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use self::token_manager::SetFlowLimitAccounts;
use crate::instruction::{self, InterchainTokenServiceInstruction};
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, check_program_account, Roles};

pub(crate) mod gmp;
pub(crate) mod interchain_token;
pub(crate) mod interchain_transfer;
pub(crate) mod link_token;
pub(crate) mod token_manager;

/// Processes an instruction.
///
/// # Errors
///
/// A `ProgramError` containing the error that occurred is returned. Log
/// messages are also generated with more detailed information.
#[allow(clippy::too_many_lines)]
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    check_program_account(*program_id)?;
    let instruction = match InterchainTokenServiceInstruction::try_from_slice(instruction_data) {
        Ok(instruction) => instruction,
        Err(err) => {
            msg!("Failed to deserialize instruction: {:?}", err);
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    match instruction {
        InterchainTokenServiceInstruction::Initialize {
            chain_name,
            its_hub_address,
        } => process_initialize(program_id, accounts, chain_name, its_hub_address),
        InterchainTokenServiceInstruction::SetPauseStatus { paused } => {
            process_set_pause_status(accounts, paused)
        }
        InterchainTokenServiceInstruction::ItsGmpPayload { message } => {
            gmp::process_inbound(accounts, message)
        }
        InterchainTokenServiceInstruction::SetTrustedChain { chain_name } => {
            process_set_trusted_chain(accounts, chain_name)
        }
        InterchainTokenServiceInstruction::RemoveTrustedChain { chain_name } => {
            process_remove_trusted_chain(accounts, &chain_name)
        }
        InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
            destination_minter,
        } => interchain_token::approve_deploy_remote_interchain_token(
            accounts,
            deployer,
            salt,
            &destination_chain,
            &destination_minter,
        ),
        InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
        } => interchain_token::revoke_deploy_remote_interchain_token(
            accounts,
            deployer,
            salt,
            &destination_chain,
        ),
        InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken => {
            link_token::register_canonical_interchain_token(accounts)
        }
        InterchainTokenServiceInstruction::DeployRemoteCanonicalInterchainToken {
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_canonical_interchain_token(
            accounts,
            destination_chain,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::DeployInterchainToken {
            salt,
            name,
            symbol,
            decimals,
        } => interchain_token::process_deploy(accounts, salt, name, symbol, decimals),
        InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token(
            accounts,
            salt,
            destination_chain,
            None,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter {
            salt,
            destination_chain,
            destination_minter,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token(
            accounts,
            salt,
            destination_chain,
            Some(destination_minter),
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::InterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
        } => interchain_transfer::process_outbound_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            None,
            None,
        ),
        InterchainTokenServiceInstruction::RegisterTokenMetadata {
            gas_value,
            signing_pda_bump,
        } => link_token::register_token_metadata(accounts, gas_value, signing_pda_bump),
        InterchainTokenServiceInstruction::RegisterCustomToken {
            salt,
            token_manager_type,
            operator,
        } => link_token::register_custom_token(accounts, salt, token_manager_type, operator),
        InterchainTokenServiceInstruction::LinkToken {
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        } => link_token::process_outbound(
            accounts,
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::SetFlowLimit { flow_limit } => {
            let mut instruction_accounts = SetFlowLimitAccounts::try_from(accounts)?;

            msg!("Instruction: SetFlowLimit");
            ensure_signer_roles(
                &crate::id(),
                instruction_accounts.its_root_pda,
                instruction_accounts.flow_limiter,
                instruction_accounts.its_user_roles_pda,
                Roles::OPERATOR,
            )?;

            instruction_accounts.flow_limiter = instruction_accounts.its_root_pda;
            token_manager::set_flow_limit(&instruction_accounts, flow_limit)
        }
        InterchainTokenServiceInstruction::OperatorInstruction(operator_instruction) => {
            process_operator_instruction(accounts, operator_instruction)
        }
        InterchainTokenServiceInstruction::TokenManagerInstruction(token_manager_instruction) => {
            token_manager::process_instruction(accounts, token_manager_instruction)
        }
        InterchainTokenServiceInstruction::InterchainTokenInstruction(
            interchain_token_instruction,
        ) => interchain_token::process_instruction(accounts, interchain_token_instruction),
        InterchainTokenServiceInstruction::CallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            gas_value,
            signing_pda_bump,
        } => interchain_transfer::process_outbound_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            Some(data),
            None,
        ),
        InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData {
            token_id,
            destination_chain,
            destination_address,
            amount,
            payload_hash,
            gas_value,
            signing_pda_bump,
        } => interchain_transfer::process_outbound_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            None,
            Some(payload_hash),
        ),
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    chain_name: String,
    its_hub_address: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let program_data_account = next_account_info(account_info_iter)?;
    let gateway_root_pda_account = next_account_info(account_info_iter)?;
    let its_root_pda_account = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;
    let operator = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    msg!("Instruction: Initialize");
    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check: Upgrade Authority
    ensure_upgrade_authority(program_id, payer, program_data_account)?;

    // Check: PDA Account is not initialized
    its_root_pda_account.check_uninitialized_pda()?;

    // Check: Gateway Root PDA Account is valid.
    let gateway_config_data = gateway_root_pda_account.try_borrow_data()?;
    let gateway_config =
        GatewayConfig::read(&gateway_config_data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
    axelar_solana_gateway::assert_valid_gateway_root_pda(
        gateway_config.bump,
        gateway_root_pda_account.key,
    )?;

    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda(gateway_root_pda_account.key);
    let its_root_config =
        InterchainTokenService::new(its_root_pda_bump, chain_name, its_hub_address);
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

fn process_operator_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instruction::operator::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;

    let its_config = InterchainTokenService::load(role_management_accounts.resource)?;
    assert_valid_its_root_pda(
        role_management_accounts.resource,
        gateway_root_pda.key,
        its_config.bump,
    )?;

    match instruction {
        instruction::operator::Instruction::TransferOperatorship(inputs) => {
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
        instruction::operator::Instruction::ProposeOperatorship(inputs) => {
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
        instruction::operator::Instruction::AcceptOperatorship(inputs) => {
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
    let system_account = next_account_info(accounts_iter)?;

    msg!("Instruction: SetPauseStatus");
    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;
    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(
        its_root_pda,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;
    its_root_config.paused = paused;
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

fn process_set_trusted_chain(accounts: &[AccountInfo<'_>], chain_name: String) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    msg!("Instruction: SetTrustedChain");
    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;
    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(
        its_root_pda,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;

    its_root_config.add_trusted_chain(chain_name);
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

fn process_remove_trusted_chain(accounts: &[AccountInfo<'_>], chain_name: &str) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    msg!("Instruction: RemoveTrustedChain");
    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;
    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(
        its_root_pda,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;

    its_root_config.remove_trusted_chain(chain_name);
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

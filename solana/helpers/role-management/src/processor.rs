//! This module provides logic to handle user role management instructions.
use program_utils::{close_pda, init_rkyv_pda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instructions::{RoleManagementInstruction, RoleManagementInstructionInputs};
use crate::seed_prefixes;
use crate::state::{RoleProposal, Roles, UserRoles};

pub fn process<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction: RoleManagementInstruction,
) -> ProgramResult {
    let parsed_accounts = RoleManagementAccounts::try_from(accounts)?;

    match instruction {
        RoleManagementInstruction::ProposeRoles(inputs) => {
            propose(program_id, parsed_accounts, &inputs)?;
        }
        RoleManagementInstruction::AcceptRoles(inputs) => {
            accept(program_id, parsed_accounts, &inputs)?;
        }
        RoleManagementInstruction::TransferRoles(inputs) => {
            transfer(program_id, parsed_accounts, &inputs)?;
        }
        RoleManagementInstruction::AddRoles(inputs) => {
            add(program_id, parsed_accounts, &inputs)?;
        }
    }
    Ok(())
}

pub fn propose(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs,
) -> ProgramResult {
    let transfer_accounts = RoleTransferWithProposalAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.origin_user_account,
        transfer_accounts.origin_roles_account,
        inputs.roles,
    )?;

    let proposal = RoleProposal {
        roles: inputs.roles,
    };

    let Some(proposal_pda_bump) = inputs.proposal_pda_bump else {
        return Err(ProgramError::InvalidArgument);
    };

    init_rkyv_pda::<0, RoleProposal>(
        transfer_accounts.payer,
        transfer_accounts.proposal_account,
        program_id,
        transfer_accounts.system_account,
        proposal,
        &[
            seed_prefixes::ROLE_PROPOSAL_SEED,
            transfer_accounts.resource.key.as_ref(),
            transfer_accounts.origin_user_account.key.as_ref(),
            transfer_accounts.destination_user_account.key.as_ref(),
            &[proposal_pda_bump],
        ],
    )?;

    Ok(())
}

pub fn accept(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs,
) -> ProgramResult {
    let transfer_accounts = RoleTransferWithProposalAccounts::try_from(accounts)?;

    if !transfer_accounts.payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (derived_pda, _) = crate::create_user_roles_pda(
        program_id,
        transfer_accounts.resource.key,
        transfer_accounts.destination_user_account.key,
        inputs.destination_roles_pda_bump,
    );
    if *transfer_accounts.destination_roles_account.key != derived_pda {
        msg!("Derived PDA doesn't match destination roles account");
        return Err(ProgramError::InvalidArgument);
    }

    let Some(proposal_pda_bump) = inputs.proposal_pda_bump else {
        return Err(ProgramError::InvalidArgument);
    };

    let (derived_proposal_pda, _) = crate::create_roles_proposal_pda(
        program_id,
        transfer_accounts.resource.key,
        transfer_accounts.origin_user_account.key,
        transfer_accounts.destination_user_account.key,
        proposal_pda_bump,
    );

    if derived_proposal_pda != *transfer_accounts.proposal_account.key {
        msg!("Derived PDA doesn't match given  proposal account address");
        return Err(ProgramError::InvalidArgument);
    }

    let proposal = RoleProposal::load(program_id, transfer_accounts.proposal_account)?;
    if !proposal.roles.contains(inputs.roles) {
        msg!("Trying to accept a role that hasn't been proposed");
        return Err(ProgramError::InvalidArgument);
    }

    close_pda(
        transfer_accounts.origin_user_account,
        transfer_accounts.proposal_account,
    )?;

    transfer_roles(
        program_id,
        &transfer_accounts.into(),
        inputs.roles,
        inputs.destination_roles_pda_bump,
    )?;

    Ok(())
}

fn transfer_roles(
    program_id: &Pubkey,
    accounts: &RoleTransferAccounts<'_>,
    roles: Roles,
    destination_roles_pda_bump: u8,
) -> ProgramResult {
    let mut origin_user_roles = UserRoles::load(program_id, accounts.origin_roles_account)?;
    origin_user_roles.remove(roles);
    origin_user_roles.store(accounts.origin_roles_account)?;

    if let Ok(mut destination_user_roles) =
        UserRoles::load(program_id, accounts.destination_roles_account)
    {
        destination_user_roles.add(roles);
        destination_user_roles.store(accounts.destination_roles_account)?;
    } else {
        UserRoles::new(roles, destination_roles_pda_bump).init(
            program_id,
            accounts.system_account,
            accounts.payer,
            accounts.resource,
            accounts.destination_user_account,
            accounts.destination_roles_account,
        )?;
    }
    Ok(())
}

pub fn transfer(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs,
) -> ProgramResult {
    let transfer_accounts = RoleTransferAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.origin_user_account,
        transfer_accounts.origin_roles_account,
        inputs.roles,
    )?;

    transfer_roles(
        program_id,
        &transfer_accounts,
        inputs.roles,
        inputs.destination_roles_pda_bump,
    )?;

    Ok(())
}

pub fn add(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs,
) -> ProgramResult {
    let add_accounts = RoleAddAccounts::try_from(accounts)?;
    ensure_signer_roles(
        program_id,
        add_accounts.resource,
        add_accounts.payer,
        add_accounts.origin_roles_account,
        inputs.roles,
    )?;

    if let Ok(mut destination_user_roles) =
        UserRoles::load(program_id, add_accounts.destination_roles_account)
    {
        destination_user_roles.add(inputs.roles);
        destination_user_roles.store(add_accounts.destination_roles_account)?;
    } else {
        UserRoles::new(inputs.roles, inputs.destination_roles_pda_bump).init(
            program_id,
            add_accounts.system_account,
            add_accounts.payer,
            add_accounts.resource,
            add_accounts.destination_user_account,
            add_accounts.destination_roles_account,
        )?;
    }

    Ok(())
}

fn ensure_roles(
    program_id: &Pubkey,
    resource: &AccountInfo<'_>,
    user: &AccountInfo<'_>,
    roles_account: &AccountInfo<'_>,
    roles: Roles,
) -> ProgramResult {
    let user_roles = UserRoles::load(program_id, roles_account)?;
    if !user_roles.contains(roles) {
        return Err(ProgramError::InvalidArgument);
    }
    let (derived_pda, _) =
        crate::create_user_roles_pda(program_id, resource.key, user.key, user_roles.bump());

    if *roles_account.key != derived_pda {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

fn ensure_signer_roles(
    program_id: &Pubkey,
    resource: &AccountInfo<'_>,
    signer: &AccountInfo<'_>,
    roles_account: &AccountInfo<'_>,
    roles: Roles,
) -> ProgramResult {
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    ensure_roles(program_id, resource, signer, roles_account, roles)
}

pub struct RoleManagementAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    origin_roles_account: &'a AccountInfo<'a>,
    destination_roles_account: Option<&'a AccountInfo<'a>>,
    origin_user_account: Option<&'a AccountInfo<'a>>,
    destination_user_account: Option<&'a AccountInfo<'a>>,
    proposal_account: Option<&'a AccountInfo<'a>>,
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for RoleManagementAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error> {
        let account_iter = &mut value.iter();
        Ok(Self {
            system_account: next_account_info(account_iter)?,
            payer: next_account_info(account_iter)?,
            resource: next_account_info(account_iter)?,
            origin_roles_account: next_account_info(account_iter)?,
            destination_roles_account: next_account_info(account_iter).ok(),
            origin_user_account: next_account_info(account_iter).ok(),
            destination_user_account: next_account_info(account_iter).ok(),
            proposal_account: next_account_info(account_iter).ok(),
        })
    }
}

pub(crate) struct RoleTransferAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    origin_roles_account: &'a AccountInfo<'a>,
    destination_roles_account: &'a AccountInfo<'a>,
    origin_user_account: &'a AccountInfo<'a>,
    destination_user_account: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<RoleManagementAccounts<'a>> for RoleTransferAccounts<'a> {
    type Error = ProgramError;
    fn try_from(value: RoleManagementAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            system_account: value.system_account,
            payer: value.payer,
            resource: value.resource,
            origin_roles_account: value.origin_roles_account,
            destination_roles_account: value
                .destination_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_user_account: value
                .origin_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            destination_user_account: value
                .destination_user_account
                .ok_or(ProgramError::InvalidArgument)?,
        })
    }
}

pub(crate) struct RoleTransferWithProposalAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    origin_roles_account: &'a AccountInfo<'a>,
    destination_roles_account: &'a AccountInfo<'a>,
    origin_user_account: &'a AccountInfo<'a>,
    destination_user_account: &'a AccountInfo<'a>,
    proposal_account: &'a AccountInfo<'a>,
}

pub(crate) type RoleAddAccounts<'a> = RoleTransferAccounts<'a>;

impl<'a> TryFrom<RoleManagementAccounts<'a>> for RoleTransferWithProposalAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: RoleManagementAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            system_account: value.system_account,
            payer: value.payer,
            resource: value.resource,
            origin_roles_account: value.origin_roles_account,
            destination_roles_account: value
                .destination_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_user_account: value
                .origin_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            destination_user_account: value
                .destination_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            proposal_account: value
                .proposal_account
                .ok_or(ProgramError::InvalidArgument)?,
        })
    }
}

impl<'a> From<RoleTransferWithProposalAccounts<'a>> for RoleTransferAccounts<'a> {
    fn from(value: RoleTransferWithProposalAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            resource: value.resource,
            origin_roles_account: value.origin_roles_account,
            destination_roles_account: value.destination_roles_account,
            origin_user_account: value.origin_user_account,
            destination_user_account: value.destination_user_account,
        }
    }
}
//! Executes all logic for processing a `ExecuteOperatorProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L75).
use program_utils::check_rkyv_initialized_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::ensure_valid_governance_root_pda;
use crate::events::GovernanceEvent;
use crate::state::proposal::{ArchivedExecutableProposal, ExecutableProposal, ExecuteProposalData};
use crate::state::{operator, GovernanceConfig};

/// Executes a previously proposal whitelisted for execution by the operator.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    execute_proposal_data: &ExecuteProposalData,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _payer = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;
    let operator_account = next_account_info(accounts_iter)?;
    let operator_pda_marker_account = next_account_info(accounts_iter)?;

    let account_data = config_pda.try_borrow_data()?;
    let config_data =
        check_rkyv_initialized_pda::<GovernanceConfig>(program_id, config_pda, &account_data)?;
    ensure_valid_governance_root_pda(config_data.bump, config_pda.key)?;

    // Only the operator account can execute the proposal.
    if !operator_account.is_signer || operator_account.key.to_bytes() != config_data.operator {
        msg!("Operator account must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let hash = ExecutableProposal::calculate_hash(
        &Pubkey::new_from_array(execute_proposal_data.target_address),
        &execute_proposal_data.call_data,
        &execute_proposal_data.native_value,
    );

    ExecutableProposal::ensure_correct_proposal_pda(
        proposal_account.key,
        &hash,
        execute_proposal_data.call_data.proposal_bump()?,
    )?;

    operator::ensure_correct_managed_proposal_pda(
        operator_pda_marker_account.key,
        &hash,
        execute_proposal_data
            .call_data
            .proposal_operator_managed_bump()?,
    )?;

    // Binding, so we can drop the account_data borrow before the CPI call.
    let config_bump = config_data.bump;

    let proposal_account_data = proposal_account.try_borrow_data()?;
    let proposal = ArchivedExecutableProposal::load_from(
        program_id,
        proposal_account,
        proposal_account_data.as_ref(),
    )?;

    // We need add this drop here to release the proposal_account_data borrow before
    // the CPI call. Todo: We can remove this drop when we have the ability to
    // borrow the account data for a specific scope, probably extracting above
    // logic to its own function.
    drop(account_data);

    // Only invoke with target program accounts.
    let mut target_program_accounts = accounts
        .get(4..)
        .ok_or(ProgramError::InvalidInstructionData)?
        .as_ref()
        .to_vec();
    target_program_accounts.push(config_pda.clone());

    proposal.unchecked_execute(
        &target_program_accounts,
        config_pda,
        config_bump,
        Pubkey::new_from_array(execute_proposal_data.target_address),
        execute_proposal_data.call_data.clone(),
        execute_proposal_data.find_target_native_value_account_info(accounts),
        execute_proposal_data.native_value()?,
    )?;

    // Send event
    let event = GovernanceEvent::OperatorProposalExecuted {
        hash,
        target_address: execute_proposal_data.target_address,
        call_data: execute_proposal_data
            .call_data
            .to_bytes()
            .expect("Should serialize call data"),
        native_value: execute_proposal_data.native_value,
    };
    event.emit()?;
    drop(proposal_account_data);
    ArchivedExecutableProposal::remove(proposal_account, config_pda)?;
    program_utils::close_pda(config_pda, operator_pda_marker_account)
}

//! Holds all logic for processing a `ScheduleTimeLockProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L15).

use program_utils::{checked_from_u256_le_bytes_to_u64, current_time, validate_system_account_key, from_u64_to_u256_le_bytes};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::state::operator;
use crate::state::proposal::ExecutableProposal;

/// Processes a Governance GMP `ScheduleTimeLockProposal` command.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    ctx: ProcessGMPContext,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let _root_pda = next_account_info(accounts_iter)?;
    let proposal_pda = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;

    let proposal_time = checked_from_u256_le_bytes_to_u64(&ctx.cmd_payload.eta.to_le_bytes())?;

    let proposal_time =
        at_least_default_eta_delay(proposal_time, ctx.minimum_eta_delay).map_err(|err| {
            msg!(
                "Cannot enforce default eta delay due to an error. Tried eta: {}: err was: {}",
                proposal_time,
                err
            );
            ProgramError::InvalidArgument
        })?;

    // Forge the new proposal
    let (pubkey, bump) = ExecutableProposal::pda(&ctx.proposal_hash);
    if pubkey != *proposal_pda.key {
        msg!("Derived proposal PDA does not match provided one");
        return Err(ProgramError::InvalidArgument);
    }
    let managed_bump = operator::derive_managed_proposal_pda(&ctx.proposal_hash).1;
    let proposal = ExecutableProposal::new(proposal_time, bump, managed_bump);

    // Store proposal
    proposal.store(
        system_account,
        payer,
        proposal_pda,
        &ctx.proposal_hash,
        bump,
    )?;

    // Send event
    let event = GovernanceEvent::ProposalScheduled {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
        eta: from_u64_to_u256_le_bytes(proposal_time),
    };

    event.emit()
}

// Enforce config ETA delay in case input eta is below.
fn at_least_default_eta_delay(proposal_time: u64, min_eta_delay: u64) -> Result<u64, ProgramError> {
    let now = current_time()?;
    let minimum_proposal_eta = now
        .checked_add(min_eta_delay)
        .expect("Be able to add the minimum proposal eta delay to current time");
    if proposal_time < minimum_proposal_eta {
        Ok(minimum_proposal_eta)
    } else {
        Ok(proposal_time)
    }
}

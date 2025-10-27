use crate::ExecuteProposalCallData;
use alloy_sol_types::SolType;
use anchor_lang::{solana_program, AnchorDeserialize};

/// A module to convert the payload data of a governance GMP command.
use governance_gmp::alloy_primitives::Bytes;
use governance_gmp::GovernanceCommandPayload;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Decodes the payload of a governance GMP command.
///
/// # Errors
///
/// A `ProgramError` is returned if the payload cannot be deserialized.
pub fn decode_payload(
    raw_payload: &[u8],
) -> std::result::Result<GovernanceCommandPayload, ProgramError> {
    GovernanceCommandPayload::abi_decode(raw_payload, true).map_err(|err| {
        msg!("Cannot abi decode GovernanceCommandPayload: {}", err);
        ProgramError::InvalidArgument
    })
}

/// Decodes the target address from the payload.
///
/// # Errors
///
/// A `ProgramError` is returned if the target address cannot be deserialized.
pub fn decode_payload_target(
    payload_target_addr: &Bytes,
) -> std::result::Result<Pubkey, ProgramError> {
    let target: [u8; 32] = payload_target_addr.as_ref().try_into().map_err(|_err| {
        msg!("Cannot cast incoming target address for governance gmp command");
        ProgramError::InvalidArgument
    })?;
    Ok(Pubkey::from(target))
}

pub fn decode_payload_call_data(
    call_data: &Bytes,
) -> std::result::Result<ExecuteProposalCallData, ProgramError> {
    ExecuteProposalCallData::try_from_slice(call_data).map_err(|_| ProgramError::InvalidArgument)
}

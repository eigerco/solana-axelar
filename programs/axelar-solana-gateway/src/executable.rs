//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use crate::error::GatewayError;
use crate::state::incoming_message::{command_id, IncomingMessage};
use crate::{get_gateway_root_config_pda, get_validate_message_signing_pda, BytemuckedPda};
use axelar_solana_encoding::types::messages::Message;
use borsh::{BorshDeserialize, BorshSerialize};
use core::str::FromStr;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod axelar_payload;
pub use axelar_payload::{
    AxelarMessagePayload, AxelarMessagePayloadHash, EncodingScheme, PayloadError, SolanaAccountRepr,
};

/// Axelar executable command prefix
pub const AXELAR_EXECUTE: &[u8; 16] = b"axelar-execute__";

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 5;

#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AxelarExecuteInstruction {
    pub message: Message,
    pub payload_without_accounts: Vec<u8>,
    pub encoding_scheme: EncodingScheme,
}

/// Perform CPI call to the Axelar Gateway to ensure that the given message is
/// approved.
///
/// The check will ensure that the provided accounts are indeed the ones that
/// were originated on the source chain.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_event_authority` - Gateway event authority used to emit events
/// 4. `gateway_program_id` - Gateway Program ID
/// N. accounts required by the `DataPayload` constructor
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_message(
    accounts: &[AccountInfo<'_>],
    instruction: &AxelarExecuteInstruction,
) -> ProgramResult {
    let (relayer_prepended_accs, origin_chain_provided_accs) =
        accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
    let accounts_iter = &mut relayer_prepended_accs.iter();
    let incoming_message_pda = next_account_info(accounts_iter)?;

    let incoming_message_payload_hash;
    let signing_pda_bump = {
        // scope to drop the account borrow after reading the data we want

        // Check: Incoming Message account is owned by the Gateway
        if incoming_message_pda.owner != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        incoming_message_payload_hash = incoming_message.payload_hash;
        incoming_message.signing_pda_bump
    };

    let payload = AxelarMessagePayload::new(
        &instruction.payload_without_accounts,
        origin_chain_provided_accs,
        instruction.encoding_scheme,
    );

    // Check: Payload hash matches IncomingMessage's
    let payload_hash = payload.hash()?.0;
    if *payload_hash != incoming_message_payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check: parsed accounts matches the original chain provided accounts
    if !payload.solana_accounts().eq(origin_chain_provided_accs) {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(
        accounts,
        &instruction.message,
        &payload_hash,
        signing_pda_bump,
    )
}

/// Perform CPI (Cross-Program Invocation) call to the Axelar Gateway to
/// ensure that the given command (containing a GMP message) is approved
///
/// This is useful for contracts that have custom legacy implementations by
/// Axelar on other chains, and therefore they cannot provide the accounts in
/// the GMP message. Therefore, the validation of the accounts becomes the
/// responsibility of the destination program.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_event_authority` - Gateway event authority used to emit events
/// 4. `gateway_program_id` - Gateway Program ID
/// N. accounts required by the inner instruction (part of the payload).
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_with_raw_payload(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let incoming_message_pda = next_account_info(accounts_iter)?;

    let incoming_message_payload_hash;
    let signing_pda_bump = {
        // scope to release the account after reading the data we want
        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;

        incoming_message_payload_hash = incoming_message.payload_hash;
        incoming_message.signing_pda_bump
    };

    let payload_hash = solana_program::keccak::hash(payload).to_bytes();
    if payload_hash != incoming_message_payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(accounts, message, &payload_hash, signing_pda_bump)
}

fn validate_message_internal(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload_hash: &[u8; 32],
    signing_pda_derived_bump: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let gateway_incoming_message = next_account_info(account_info_iter)?;
    let signing_pda = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let gateway_event_authority = next_account_info(account_info_iter)?;
    let gateway_program_id = next_account_info(account_info_iter)?;

    // Build the actual Message we are going to use
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

    // Check: Original message's payload_hash is equivalent to provided payload's
    // hash
    if &message.payload_hash != payload_hash {
        msg!("Invalid payload hash");
        return Err(ProgramError::InvalidInstructionData);
    }

    invoke_signed(
        &crate::instructions::validate_message(
            gateway_incoming_message.key,
            signing_pda.key,
            message.clone(),
        )?,
        &[
            gateway_incoming_message.clone(),
            signing_pda.clone(),
            gateway_root_pda.clone(),
            gateway_program_id.clone(),
            gateway_event_authority.clone(),
        ],
        &[&[
            crate::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED,
            &command_id,
            &[signing_pda_derived_bump],
        ]],
    )?;

    Ok(())
}

/// # Create a generic `Execute` instruction
///
/// Intended to be used by the relayer when it is about to call the
/// destination program.
///
/// It will prepend the accounts array with these predefined accounts
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_event_authority` - Gateway event authority used to emit events
/// 4. `gateway_program_id` - Gateway Program ID
/// N... - The accounts provided in the `axelar_message_payload`
///
/// # Errors
/// - if the destination address is not a vald base58 encoded ed25519 pubkey
/// - if the `axelar_message_payload` could not be decoded
/// - if we cannot encode the `AxelarExecutablePayload`
pub fn construct_axelar_executable_ix(
    message: Message,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: &[u8],
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_incoming_message: Pubkey,
) -> Result<Instruction, ProgramError> {
    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_er| ProgramError::InvalidAccountData)?;
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let (signing_pda, _) = get_validate_message_signing_pda(destination_address, command_id);
    let gateway_root_pda = get_gateway_root_config_pda().0;
    let gateway_event_authority =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::id()).0;

    // The expected accounts for the `ValidateMessage` ix
    let mut accounts = vec![
        AccountMeta::new(gateway_incoming_message, false),
        AccountMeta::new_readonly(signing_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new_readonly(crate::id(), false),
    ];

    let payload = AxelarMessagePayload::decode(axelar_message_payload)?;

    accounts.extend(payload.account_meta());

    let instruction = AxelarExecuteInstruction {
        message,
        payload_without_accounts: payload.payload_without_accounts().to_vec(),
        encoding_scheme: payload.encoding_scheme(),
    };

    let data = serialize_instruction(&instruction)?;

    Ok(Instruction {
        program_id: destination_address,
        accounts,
        data,
    })
}

fn serialize_instruction(instruction: &AxelarExecuteInstruction) -> Result<Vec<u8>, ProgramError> {
    // In our tests, randomly generated messages have, in average, 175 bytes, so 256
    // should be sufficient to avoid reallocations.
    let mut buffer = Vec::with_capacity(256);
    buffer.extend_from_slice(AXELAR_EXECUTE);
    borsh::to_writer(&mut buffer, &instruction)
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))?;
    Ok(buffer)
}

impl TryInto<Vec<u8>> for AxelarExecuteInstruction {
    type Error = ProgramError;

    /// We prefix a byte slice with the literal contents of `AXELAR_EXECUTE` as
    /// discriminator followed by the borsh-serialized `AxelarExecuteInstruction`.
    ///
    /// This two-step approach is needed because borsh demonstrated to exhaust a Solana program's
    /// memory when trying to deserialize the alternative form (Tag, Message) for an absent tag.
    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        serialize_instruction(&self)
    }
}

impl TryInto<Vec<u8>> for &AxelarExecuteInstruction {
    type Error = ProgramError;

    /// We prefix a byte slice with the literal contents of `AXELAR_EXECUTE`
    /// followed by the borsh-serialized `AxelarExecuteInstruction`.
    ///
    /// This two-step approach is needed because borsh demonstrated to exhaust a Solana program's
    /// memory when trying to deserialize the alternative form (Tag, Message) for an absent tag.
    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        serialize_instruction(self)
    }
}

impl TryFrom<&[u8]> for AxelarExecuteInstruction {
    type Error = ProgramError;

    /// Tries to deserialize input into an `AxelarExecuteInstruction`
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidInstructionData`] in case the buffer data doesn't start with
    /// the [`AXELAR_EXECUTE`] discriminator.
    ///
    /// Returns [`ProgramError::BorshIoError`] if deserialization fails.
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if !value.starts_with(AXELAR_EXECUTE) {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Slicing: we already checked that slice's lower bound above.
        borsh::from_slice(&value[AXELAR_EXECUTE.len()..])
            .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_solana_gateway_test_fixtures::gateway::random_message;

    #[test]
    fn test_instruction_serialization() {
        let ix = AxelarExecuteInstruction {
            message: random_message(),
            payload_without_accounts: vec![0xDE, 0xAD, 0xBE, 0xEF],
            encoding_scheme: EncodingScheme::Borsh,
        };

        let serialized: Vec<u8> = (&ix).try_into().unwrap();
        let deserialized = AxelarExecuteInstruction::try_from(serialized.as_ref()).unwrap();
        assert_eq!(ix, deserialized);
    }
}

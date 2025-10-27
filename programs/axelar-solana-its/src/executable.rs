//! Module that defines the struct used by contracts adhering to the `AxelarInterchainTokenExecutable` interface.

use axelar_solana_gateway::executable::AxelarMessagePayload;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::accounts::AxelarInterchainTokenExecutableAccounts;
use crate::assert_valid_interchain_transfer_execute_pda;

/// Axelar Interchain Token Executable command prefix
pub(crate) const AXELAR_INTERCHAIN_TOKEN_EXECUTE: &[u8; 16] = b"axelar-its-exec_";

/// This is the payload that the `executeWithInterchainToken` processor on the destination program
/// must expect
#[derive(Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct AxelarInterchainTokenExecuteInstruction {
    /// The unique message id.
    pub command_id: [u8; 32],

    /// The source chain of the token transfer.
    pub source_chain: String,

    /// The source address of the token transfer.
    pub source_address: Vec<u8>,

    /// The destination program
    pub destination_address: Pubkey,

    /// The token ID.
    pub token_id: [u8; 32],

    /// The token (mint) address (Pubkey).
    pub token: [u8; 32],

    /// Amount of tokens being transferred.
    pub amount: u64,

    /// The execution payload
    pub data: Vec<u8>,
}

impl AxelarInterchainTokenExecuteInstruction {
    pub fn validated_accounts<'a>(
        &self,
        accounts: &'a [AccountInfo<'a>],
    ) -> Result<&'a [AccountInfo<'a>], ProgramError> {
        let accounts = AxelarInterchainTokenExecutableAccounts::try_from(accounts)?;

        if !accounts.interchain_transfer_execute.is_signer {
            msg!(
                "Signing PDA account must be a signer: {}",
                accounts.interchain_transfer_execute.key
            );
            return Err(ProgramError::MissingRequiredSignature);
        }

        assert_valid_interchain_transfer_execute_pda(
            accounts.interchain_transfer_execute,
            &self.destination_address,
        )?;

        let inner_payload = AxelarMessagePayload::decode(self.data.as_ref())?;
        if !inner_payload
            .solana_accounts()
            .eq(accounts.destination_program_accounts)
        {
            msg!("The list of accounts is different than expected");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(accounts.destination_program_accounts)
    }
}

impl TryInto<Vec<u8>> for AxelarInterchainTokenExecuteInstruction {
    type Error = ProgramError;

    /// We prefix a byte slice with the literal contents of `AXELAR_INTERCHAIN_TOKEN_EXECUTE` as
    /// discriminator followed by the borsh-serialized `AxelarInterchainTokenExecuteInstruction`.
    ///
    /// This two-step approach is needed because borsh demonstrated to exhaust a Solana program's
    /// memory when trying to deserialize the alternative form (Tag, Message) for an absent tag.
    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        serialize_instruction(&self)
    }
}

impl TryInto<Vec<u8>> for &AxelarInterchainTokenExecuteInstruction {
    type Error = ProgramError;

    /// We prefix a byte slice with the literal contents of `AXELAR_INTERCHAIN_TOKEN_EXECUTE`
    /// followed by the borsh-serialized `AxelarInterchainTokenExecuteInstruction`.
    ///
    /// This two-step approach is needed because borsh demonstrated to exhaust a Solana program's
    /// memory when trying to deserialize the alternative form (Tag, Message) for an absent tag.
    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        serialize_instruction(self)
    }
}

impl TryFrom<&[u8]> for AxelarInterchainTokenExecuteInstruction {
    type Error = ProgramError;

    /// Tries to deserialize input into an `AxelarInterchainTokenExecuteInstruction`
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidInstructionData`] in case the buffer data doesn't start with
    /// the [`AXELAR_INTERCHAIN_TOKEN_EXECUTE`] discriminator.
    ///
    /// Returns [`ProgramError::BorshIoError`] if deserialization fails.
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if !value.starts_with(AXELAR_INTERCHAIN_TOKEN_EXECUTE) {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Slicing: we already checked that slice's lower bound above.
        borsh::from_slice(&value[AXELAR_INTERCHAIN_TOKEN_EXECUTE.len()..])
            .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))
    }
}

fn serialize_instruction(
    instruction: &AxelarInterchainTokenExecuteInstruction,
) -> Result<Vec<u8>, ProgramError> {
    // In our tests, randomly generated messages have, in average, 175 bytes, so 256
    // should be sufficient to avoid reallocations.
    let mut buffer = Vec::with_capacity(256);
    buffer.extend_from_slice(AXELAR_INTERCHAIN_TOKEN_EXECUTE);
    borsh::to_writer(&mut buffer, &instruction)
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))?;
    Ok(buffer)
}

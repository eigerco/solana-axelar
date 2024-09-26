//! Instructions supported by the multicall program.

use std::error::Error;

use axelar_message_primitives::DestinationProgramId;
use axelar_rkyv_encoding::types::CrossChainId;
use gateway::hasher_impl;
use interchain_token_transfer_gmp::GMPPayload;
use rkyv::bytecheck::EnumCheckError;
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Instructions supported by the multicall program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    /// 3. [] system program id
    Initialize {
        /// The pda bump for the ITS root PDA
        pda_bump: u8,
    },

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    /// 3..N Accounts depend on the inner ITS instruction.
    ItsGmpPayload {
        /// The GMP payload
        abi_payload: Vec<u8>,

        /// The cross-chain id
        cross_chain_id: CrossChainId,

        /// Address of the source contract
        source_address: String,

        /// Address of the destination contract
        destination_address: String,

        /// Id of the destination chain
        destination_chain: String,
    },
}

impl InterchainTokenServiceInstruction {
    /// Serializes the instruction into a byte array.
    ///
    /// # Errors
    ///
    /// If serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let bytes = rkyv::to_bytes::<_, 0>(self).map_err(Box::new)?;

        Ok(bytes.to_vec())
    }

    /// Deserializes the instruction from a byte array.
    ///
    /// # Errors
    ///
    /// If deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // SAFETY:
        // - The byte slice represents an archived object
        // - The root of the object is stored at the end of the slice
        let bytes = unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }.map_err(Box::new)?;

        Ok(bytes)
    }
}

impl ArchivedInterchainTokenServiceInstruction {
    /// Interprets the given slice as an archived instruction.
    ///
    /// # Errors
    ///
    /// If validation fails.
    pub fn from_archived_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<EnumCheckError<u8>, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<InterchainTokenServiceInstruction>(bytes)
    }
}

/// Creates a [`InterchainTokenServiceInstruction::Initialize`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn initialize(
    payer: &Pubkey,
    gateway_root_pda: &Pubkey,
    its_root_pda: &(Pubkey, u8),
) -> Result<Instruction, ProgramError> {
    let instruction = InterchainTokenServiceInstruction::Initialize {
        pda_bump: its_root_pda.1,
    };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new(its_root_pda.0, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates a [`InterchainTokenServiceInstruction::ItsGmpPayload`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn its_gmp_payload(
    gateway_approved_message_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    abi_payload: Vec<u8>,
    cross_chain_id: CrossChainId,
    source_address: String,
    destination_address: String,
    destination_chain: String,
) -> Result<Instruction, ProgramError> {
    let mut its_accounts = derive_its_accounts(&abi_payload)?;
    let command_id = cross_chain_id.command_id(hasher_impl());
    let destination_program = DestinationProgramId(crate::id());
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);
    let (its_root_pda, _) = crate::its_root_pda(gateway_root_pda);

    let instruction = InterchainTokenServiceInstruction::ItsGmpPayload {
        abi_payload,
        cross_chain_id,
        source_address,
        destination_address,
        destination_chain,
    };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let mut accounts = vec![
        AccountMeta::new(*gateway_approved_message_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new(its_root_pda, false),
    ];
    accounts.append(&mut its_accounts);

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

// TODO: Derive the accounts required for the ITS transaction.
fn derive_its_accounts(abi_payload: &[u8]) -> Result<Vec<AccountMeta>, ProgramError> {
    match GMPPayload::decode(abi_payload) {
        Ok(GMPPayload::InterchainTransfer(_transfer_data)) => {}
        Ok(GMPPayload::DeployTokenManager(_token_manager_data)) => {}
        Ok(GMPPayload::DeployInterchainToken(_interchain_token_data)) => {}
        Err(_) => {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    Ok(vec![])
}
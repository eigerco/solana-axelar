//! Program state processor.

use std::borrow::Cow;

use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::check_program_account;
use crate::error::GatewayError;
use crate::instructions::GatewayInstruction;

mod approve_messages;
mod call_contract;
mod initialize_command;
mod initialize_config;
mod initialize_payload_verification_session;
mod rotate_signers;
mod transfer_operatorship;
mod validate_message;
mod verify_signature;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = GatewayInstruction::try_from_slice(input)?;
        check_program_account(*program_id)?;

        match instruction {
            GatewayInstruction::ApproveMessages {} => {
                msg!("Instruction: Approve Messages");
                Self::process_approve_messages(program_id, accounts)
            }
            GatewayInstruction::RotateSigners {} => {
                msg!("Instruction: Rotate Signers");
                Self::process_rotate_signers(program_id, accounts)
            }
            GatewayInstruction::CallContract {
                destination_chain,
                destination_contract_address,
                payload,
            } => {
                msg!("Instruction: Call Contract");
                Self::process_call_contract(
                    program_id,
                    accounts,
                    destination_chain,
                    destination_contract_address,
                    payload,
                )
            }
            GatewayInstruction::InitializeConfig(init_config) => {
                msg!("Instruction: Initialize Config");
                Self::process_initialize_config(program_id, accounts, init_config)
            }

            GatewayInstruction::InitializePayloadVerificationSession {
                payload_merkle_root,
                bump_seed,
            } => {
                msg!("Instruction: Initialize Verification Session");
                Self::process_initialize_payload_verification_session(
                    program_id,
                    accounts,
                    payload_merkle_root,
                    bump_seed,
                )
            }

            GatewayInstruction::VerifySignature {
                payload_merkle_root,
                verifier_set_leaf_node,
                verifier_merkle_proof,
                signature,
            } => {
                msg!("Instruction: Verify Signature");
                // Convert proxy types
                let verifier_merkle_proof: MerkleProof<SolanaSyscallHasher> =
                    MerkleProof::from_bytes(&verifier_merkle_proof)
                        .map_err(|_| ProgramError::InvalidArgument)?;

                Self::process_verify_signature(
                    program_id,
                    accounts,
                    payload_merkle_root,
                    verifier_set_leaf_node.into(),
                    verifier_merkle_proof,
                    signature.into(),
                )
            }
        }
    }
}

/// Trait for types that can representing themselves as a slice of bytes.
///
/// This trait allows for more flexible bounds on `init_pda_with_dynamic_size`,
/// reducing its dependency on `borsh`.
pub trait ToBytes {
    /// Tries to serialize `self` into a slice of bytes.
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, GatewayError>;
}

impl<T> ToBytes for T
where
    T: BorshSerialize,
{
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, GatewayError> {
        borsh::to_vec(self)
            .map_err(|_| GatewayError::ByteSerializationError)
            .map(Cow::Owned)
    }
}
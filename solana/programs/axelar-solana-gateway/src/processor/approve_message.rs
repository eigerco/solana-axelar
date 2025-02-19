use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::MerkleisedMessage;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::state::incoming_message::IncomingMessage;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::assert_valid_signature_verification_pda;

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: `https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84`
    /// # Errors
    ///
    /// Returns an error if:
    /// * Account Validation:
    ///   * Account iteration fails when extracting accounts
    ///   * Gateway Root PDA is not initialized
    ///   * Verification session PDA is not initialized
    ///   * Incoming message PDA is already initialized
    ///
    /// * Data Access and Serialization:
    ///   * Failed to borrow verification session or incoming message account data
    ///   * Verification session or incoming message data has invalid byte length
    ///
    /// * Verification Failures:
    ///   * Signature verification PDA validation fails
    ///   * Signature verification session is not valid
    ///   * Merkle proof is invalid
    ///   * Leaf node is not part of the provided merkle root
    ///
    /// * Message Processing:
    ///   * Failed to initialize PDA for incoming message
    ///   * Destination address is invalid and cannot be converted to a `Pubkey`
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * Converting `IncomingMessage::LEN` to u64 overflows.
    pub fn process_approve_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message: MerkleisedMessage,
        payload_merkle_root: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let funder = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        // No need to check the bump because that would already be implied by a valid `verification_session_account`
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: Verification session PDA is initialized.
        verification_session_account.check_initialized_pda_without_deserialization(program_id)?;
        let data = verification_session_account.try_borrow_data()?;
        let session = SignatureVerificationSessionData::read(&data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_signature_verification_pda(
            gateway_root_pda.key,
            &payload_merkle_root,
            session.bump,
            verification_session_account.key,
        )?;

        // Check: the incoming message PDA already approved
        incoming_message_pda
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::MessageAlreadyInitialised)?;

        // Check: signature verification session is complete
        if !session.signature_verification.is_valid() {
            return Err(GatewayError::SigningSessionNotValid.into());
        }

        let leaf_hash = message.leaf.hash::<SolanaSyscallHasher>();
        let proof = rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&message.proof)
            .map_err(|_err| GatewayError::InvalidMerkleProof)?;

        // Check: leaf node is part of the payload merkle root
        if !proof.verify(
            payload_merkle_root,
            &[message.leaf.position.into()],
            &[leaf_hash],
            message.leaf.set_size.into(),
        ) {
            return Err(GatewayError::LeafNodeNotPartOfMerkleRoot.into());
        }

        // crate a PDA where we write the message metadata contents
        let message = message.leaf.message;
        
        IncomingMessage::create(
            incoming_message_pda,
            funder,
            system_program,
            program_id,
            message,
        )?;

        Ok(())
    }
}

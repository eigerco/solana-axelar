//! Module for the `IncomingMessage` account type.

use axelar_solana_encoding::{hasher::SolanaSyscallHasher, types::messages::Message};
use bytemuck::{Pod, Zeroable};
use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::log::sol_log_data;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use core::str::FromStr;
use axelar_solana_encoding::LeafHash;

use crate::{assert_valid_incoming_message_pda, event_prefixes};
use crate::{error::GatewayError, get_validate_message_signing_pda, seed_prefixes};

/// Data for the incoming message (from Axelar to Solana) PDA.
#[repr(C)]
#[allow(clippy::partial_pub_fields)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct IncomingMessage {
    /// The bump that was used to create the PDA
    pub bump: u8,
    /// The bump for the signing PDA
    pub signing_pda_bump: u8,
    /// Padding for memory alignment.
    _pad: [u8; 3],
    /// Status of the message
    pub status: MessageStatus, // 1 byte
    /// Hash of the whole message
    pub message_hash: [u8; 32],
    /// Hash of the message's payload
    pub payload_hash: [u8; 32],
}

impl IncomingMessage {
    /// New default [`IncomingMessage`].
    #[must_use]
    pub fn new(
        bump: u8,
        signing_pda_bump: u8,
        status: MessageStatus,
        message_hash: [u8; 32],
        payload_hash: [u8; 32],
    ) -> Self {
        Self {
            bump,
            signing_pda_bump,
            _pad: Default::default(),
            status,
            message_hash,
            payload_hash,
        }
    }

    /// Size of this type, in bytes.
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub(crate) fn assert_valid_pda<'a>(pda: &AccountInfo<'a>, command_id: &[u8; 32]) -> Result<u8, ProgramError> {
        let seeds = [
            seed_prefixes::INCOMING_MESSAGE_SEED,
            command_id,
        ];
        let (_, bump) = Pubkey::find_program_address(
            &seeds,
            &crate::ID,
        );
        let seeds = [
            seed_prefixes::INCOMING_MESSAGE_SEED,
            command_id,
            &[bump],
        ];
        let derived_pubkey = Pubkey::create_program_address(
            &seeds,
            &crate::ID,
        )
        .expect("invalid bump for the incoming message PDA");
        if &derived_pubkey != pda.key {
            solana_program::msg!("Error: Invalid incoming message PDA ");
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(bump)
    }

    pub(crate) fn create<'a>(pda: &AccountInfo<'a>, payer: &AccountInfo<'a>, system_program: &AccountInfo<'a>, program_id: &Pubkey, message: Message) -> Result<(), ProgramError> {
        let cc_id = &message.cc_id;
        let command_id = command_id(&cc_id.chain, &cc_id.id);
        
        let bump =  Self::assert_valid_pda(pda, &command_id)?;

        let seeds = &[
            seed_prefixes::INCOMING_MESSAGE_SEED,
            &command_id,
            &[bump],
        ];
        program_utils::init_pda_raw(
            payer,
            pda,
            program_id,
            system_program,
            IncomingMessage::LEN.try_into().map_err(|_err| {
                solana_program::msg!("unexpected u64 overflow in struct size");
                ProgramError::ArithmeticOverflow
            })?,
            seeds,
        )?;

        let destination_address =
            Pubkey::from_str(&message.destination_address).map_err(|_err| {
                solana_program::msg!("Invalid destination address");
                GatewayError::InvalidDestinationAddress
            })?;
        let (_, signing_pda_bump) =
            get_validate_message_signing_pda(destination_address, command_id);

        let message_hash = message.hash::<SolanaSyscallHasher>();
        
        // Persist a new incoming message with "in progress" status in the PDA data.
        let mut data = pda.try_borrow_mut_data()?;
        let incoming_message_data =
            IncomingMessage::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        *incoming_message_data = IncomingMessage::new(
            bump,
            signing_pda_bump,
            MessageStatus::approved(),
            message_hash,
            message.payload_hash,
        );

        // Emit an event
        sol_log_data(&[
            event_prefixes::MESSAGE_APPROVED,
            &command_id,
            &destination_address.to_bytes(),
            &message.payload_hash,
            cc_id.chain.as_bytes(),
            cc_id.id.as_bytes(),
            message.source_address.as_bytes(),
            message.destination_chain.as_bytes(),
        ]);

        Ok(())
    }

    pub(crate) fn execute_approved<'a>(pda: &AccountInfo<'a>, program_id: &Pubkey, message: &Message)-> Result<([u8; 32], u8), ProgramError> {

        // compute the message hash
        let message_hash = message.hash::<SolanaSyscallHasher>();

        // compute the command id
        let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
        
        pda.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = pda.try_borrow_mut_data()?;
        let incoming_message =
            IncomingMessage::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_incoming_message_pda(
            &command_id,
            incoming_message.bump,
            pda.key,
        )?;

        // Check: message is approved
        if !incoming_message.status.is_approved() {
            return Err(GatewayError::MessageNotApproved.into());
        }

        
        // Check: message hashes match
        if incoming_message.message_hash != message_hash {
            return Err(GatewayError::MessageHasBeenTamperedWith.into());
        }

        incoming_message.status = MessageStatus::executed();

        Ok((command_id, incoming_message.bump))
    }
}

impl BytemuckedPda for IncomingMessage {}

/// If this is marked as `Approved`, the command can be used for CPI
/// [`GatewayInstructon::ValidateMessage`] instruction.
///
/// This maps to [these lines in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L636-L648)
#[repr(C)]
#[derive(Zeroable, Copy, Clone, PartialEq, Eq, Debug)]
pub struct MessageStatus(u8);

impl MessageStatus {
    /// Bit pattern: all bits zero -> Approved
    ///
    /// The state of the command after it has been approved
    #[must_use]
    pub const fn is_approved(&self) -> bool {
        self.0 == 0
    }
    /// Bit pattern: any non-zero -> Executed
    ///
    /// [`GatewayInstructon::ValidateMessage`] has been called and the command
    /// has been executed by the destination program.
    #[must_use]
    pub const fn is_executed(&self) -> bool {
        self.0 != 0
    }

    /// Creates a `MessageStatus` value which can be interpted as "approved".
    #[must_use]
    pub const fn approved() -> Self {
        Self(0)
    }

    /// Creates a `MessageStatus` value which can be interpted as "executed".
    #[must_use]
    pub const fn executed() -> Self {
        Self(1) // any non-zero value would also work
    }
}

/// SAFETY:
///
/// This implementation is sound because it satisfies all [`Pod`](^url) safety requirements.
///
/// The key point is that `MessageStatus` type (and not `bytemuck`) has the final
/// word interpreting the bit pattern for all possible states:
///    * `0`      -> Approved
///    * non-zero -> Executed
/// Therefore no invalid bit patterns are possible.
///
/// [^url]: `https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html#safety`
unsafe impl Pod for MessageStatus {}

/// Consruct a new Command ID.
/// The command id is used as a key for a message -- to prevent replay attacks.
/// It points to the storage slot that holds all metadata for a message.
///
/// For more info read [here](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#replay-prevention).
#[must_use]
pub fn command_id(source_chain: &str, message_id: &str) -> [u8; 32] {
    solana_program::keccak::hashv(&[source_chain.as_bytes(), b"-", message_id.as_bytes()]).0
}

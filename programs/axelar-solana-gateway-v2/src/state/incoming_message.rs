use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use bytemuck::{Pod, Zeroable};
use udigest::{encoding::EncodeValue, Digestable};

#[account(zero_copy)]
#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::pub_underscore_fields)]
pub struct IncomingMessage {
    pub bump: u8,
    pub signing_pda_bump: u8,
    pub _pad: [u8; 3],
    pub status: MessageStatus,
    pub message_hash: [u8; 32],
    pub payload_hash: [u8; 32],
}

impl IncomingMessage {
    pub const SEED_PREFIX: &'static [u8] = b"incoming message";

    pub fn find_pda(command_id: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX, command_id], &crate::ID)
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Pod, Zeroable)]
pub struct MessageStatus(u8);

impl MessageStatus {
    /// Creates a `MessageStatus` value which can be interpreted as "approved".
    #[must_use]
    pub const fn approved() -> Self {
        Self(0)
    }

    pub const fn executed() -> Self {
        Self(1)
    }

    pub const fn is_approved(&self) -> bool {
        self.0 == 0
    }

    pub const fn is_executed(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, PartialEq, Eq, Digestable, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct MessageLeaf {
    /// The message contained within this leaf node.
    pub message: Message,

    /// The position of this leaf within the Merkle tree.
    pub position: u16,

    /// The total number of leaves in the Merkle tree.
    pub set_size: u16,

    /// A domain separator used to ensure the uniqueness of hashes across
    /// different contexts.
    pub domain_separator: [u8; 32],
}

pub(crate) struct VecBuf(pub(crate) Vec<u8>);
impl udigest::encoding::Buffer for VecBuf {
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

impl MessageLeaf {
    pub fn hash(&self) -> [u8; 32] {
        let mut buffer = VecBuf(vec![]);
        self.unambiguously_encode(EncodeValue::new(&mut buffer));
        solana_program::keccak::hash(&buffer.0).to_bytes()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MerkleisedMessage {
    /// The leaf node representing the message in the Merkle tree.
    pub leaf: MessageLeaf,

    /// The Merkle proof demonstrating the message's inclusion in the payload's
    /// Merkle tree.
    pub proof: Vec<u8>,
}

/// Identifies a specific blockchain and its unique identifier within that
/// chain.
#[derive(Clone, PartialEq, Eq, Digestable, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CrossChainId {
    /// The name or identifier of the source blockchain.
    pub chain: String,

    /// A unique identifier within the specified blockchain.
    pub id: String,
}

/// Represents a message intended for cross-chain communication.
#[derive(Clone, PartialEq, Digestable, Eq, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct Message {
    /// The cross-chain identifier of the message
    pub cc_id: CrossChainId,

    /// The source address from which the message originates.
    pub source_address: String,

    /// The destination blockchain where the message is intended to be sent.
    pub destination_chain: String,

    /// The destination address on the target blockchain.
    pub destination_address: String,

    /// A 32-byte hash of the message payload, ensuring data integrity.
    pub payload_hash: [u8; 32],
}

impl Message {
    pub fn hash(&self) -> [u8; 32] {
        let mut buffer = VecBuf(vec![]);
        self.unambiguously_encode(EncodeValue::new(&mut buffer));
        solana_program::keccak::hash(&buffer.0).to_bytes()
    }

    pub fn command_id(&self) -> [u8; 32] {
        let cc_id = &self.cc_id;
        solana_program::keccak::hashv(&[cc_id.chain.as_bytes(), b"-", cc_id.id.as_bytes()]).0
    }
}

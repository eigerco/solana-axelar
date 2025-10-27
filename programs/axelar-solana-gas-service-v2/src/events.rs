//! Events emitted by the Axelar Solana Gas service
//!
//! All events have optional fields for SPL tokens, but SPL tokens
//! are not currently supported.

use anchor_lang::prelude::{
    borsh, event, AnchorDeserialize, AnchorSerialize, Discriminator, Pubkey,
};

type MessageId = String;

/// Event emitted by the Axelar Solana Gas service
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GasServiceEvent {
    /// Event when SOL was used to pay for a contract call
    GasPaid(GasPaidEvent),
    /// Event when SOL was added to fund an already emitted contract call
    GasAdded(GasAddedEvent),
    /// Event when SOL was refunded
    GasRefunded(GasRefundedEvent),
    /// Event when SOL was collected
    GasCollected(GasCollectedEvent),
}

/// Represents the event emitted when gas is paid for a contract call.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasPaidEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The amount of SOL paid
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// Optional SPL token account (sender)
    pub spl_token_account: Option<Pubkey>,
}

/// Represents the event emitted when gas is added.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasAddedEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Message Id
    pub message_id: MessageId,
    /// The amount of SOL added
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// Optional SPL token account (sender)
    pub spl_token_account: Option<Pubkey>,
}

/// Represents the event emitted when gas is refunded.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasRefundedEvent {
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// Message Id
    pub message_id: MessageId,
    /// The amount of SOL refunded
    pub amount: u64,
    /// Optional SPL token account (receiver)
    pub spl_token_account: Option<Pubkey>,
}

/// Represents the event emitted when accumulated gas is collected.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasCollectedEvent {
    /// The receiver of the gas
    pub receiver: Pubkey,
    /// The amount of SOL refunded
    pub amount: u64,
    /// Optional SPL token account (receiver)
    pub spl_token_account: Option<Pubkey>,
}

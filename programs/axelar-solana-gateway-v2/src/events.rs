use crate::u256::U256;
use anchor_lang::prelude::*;

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageApprovedEvent {
    pub command_id: [u8; 32],
    pub destination_address: Pubkey,
    pub payload_hash: [u8; 32],
    pub source_chain: String,
    pub cc_id: String,
    pub source_address: String,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageExecutedEvent {
    pub command_id: [u8; 32],
    pub destination_address: Pubkey,
    pub payload_hash: [u8; 32],
    pub source_chain: String,
    pub cc_id: String,
    pub source_address: String,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VerifierSetRotatedEvent {
    pub epoch: U256,
    pub verifier_set_hash: [u8; 32],
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallContractEvent {
    pub sender: Pubkey,
    pub payload_hash: [u8; 32],
    pub destination_chain: String,
    pub destination_contract_address: String,
    pub payload: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorshipTransferredEvent {
    pub new_operator: [u8; 32],
}

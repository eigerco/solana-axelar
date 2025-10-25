//! All PDAs owned by the memo program
use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};

/// A counter PDA that keeps track of how many memos have been received from the
/// gateway
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Payload {
    /// the counter of how many memos have been received from the gateway
    pub storage_id: u64,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Storage {
    pub value: String,
    pub bump: u8,
}
use anchor_lang::prelude::*;

#[event]
pub struct ProposalScheduled {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
    pub eta: u64,
}

#[event]
pub struct OperatorProposalApproved {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
}

#[event]
pub struct OperatorProposalCancelled {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
}

#[event]
pub struct ProposalCancelled {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
    pub eta: u64,
}

#[event]
pub struct ProposalExecuted {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
    pub eta: u64,
}

#[event]
pub struct OperatorProposalExecuted {
    pub hash: [u8; 32],
    pub target_address: Vec<u8>,
    pub call_data: Vec<u8>,
    pub native_value: Vec<u8>,
}

#[event]
pub struct OperatorshipTransferred {
    pub old_operator: Vec<u8>,
    pub new_operator: Vec<u8>,
}

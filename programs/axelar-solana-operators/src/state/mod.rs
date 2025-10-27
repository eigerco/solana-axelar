use anchor_lang::prelude::*;

/// Registry config - holds master operator
#[account]
#[derive(InitSpace)]
pub struct OperatorRegistry {
    /// Master operator who can add/remove operators
    pub owner: Pubkey,
    /// Total number of operators
    pub operator_count: u64,
    /// Bump seed
    pub bump: u8,
}

impl OperatorRegistry {
    pub const SEED_PREFIX: &'static [u8] = b"operator_registry";
}

/// Individual operator account - holds operator pubkey
#[account]
#[derive(InitSpace)]
pub struct OperatorAccount {
    /// The operator's pubkey
    pub operator: Pubkey,
    /// Bump seed
    pub bump: u8,
}

impl OperatorAccount {
    pub const SEED_PREFIX: &'static [u8] = b"operator";
}

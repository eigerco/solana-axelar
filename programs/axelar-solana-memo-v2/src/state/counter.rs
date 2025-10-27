use anchor_lang::prelude::*;

/// A counter PDA that keeps track of how many memos have been received from the
/// gateway
#[account]
#[derive(Debug, InitSpace)]
pub struct Counter {
    /// the counter of how many memos have been received from the gateway
    pub counter: u64,
    /// Bump for the counter PDA
    pub bump: u8,
}

impl Counter {
    pub const SEED_PREFIX: &'static [u8] = b"counter";

    pub fn get_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX], &crate::ID)
    }
}

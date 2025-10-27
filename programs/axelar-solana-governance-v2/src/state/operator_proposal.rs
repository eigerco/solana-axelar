use anchor_lang::prelude::*;

#[derive(Debug, InitSpace)]
#[account]
pub struct OperatorProposal {}

impl OperatorProposal {
    pub const SEED_PREFIX: &'static [u8] = b"operator-managed-proposal";

    pub fn find_pda(proposal_hash: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX, proposal_hash], &crate::ID)
    }
}

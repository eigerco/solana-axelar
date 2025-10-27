use crate::state::user_roles::Roles;
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, PartialEq, Eq, Copy, Debug)]
/// Proposal to transfer roles to a user.
pub struct RoleProposal {
    // The roles to be transferred.
    pub roles: Roles,
    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl RoleProposal {
    pub const SEED_PREFIX: &'static [u8] = b"role-proposal";
}

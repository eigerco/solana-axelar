//! Axelar Operators program for the Solana blockchain
use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

use instructions::*;
pub use state::*;

use program_utils::ensure_single_feature;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
declare_id!("oprrZ9bgRsEftLetV4GhQ3L2fgcWixpozQfRKwnZfDJ");

#[cfg(feature = "stagenet")]
declare_id!("oprXXJdUK7Nru5JvRvGYq4v13m6WyHukWthrDHjD4wN");

#[cfg(feature = "testnet")]
declare_id!("oprmPyi5v1mR3RDPoh72H6t6kNw2dbCYX8goanVF2gq");

#[cfg(feature = "mainnet")]
declare_id!("opr1111111111111111111111111111111111111111");

#[program]
pub mod operators {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize(ctx)
    }

    pub fn add_operator(ctx: Context<AddOperator>) -> Result<()> {
        instructions::add_operator(ctx)
    }

    pub fn remove_operator(ctx: Context<RemoveOperator>) -> Result<()> {
        instructions::remove_operator(ctx)
    }

    pub fn transfer_owner(ctx: Context<TransferOwner>) -> Result<()> {
        instructions::transfer_owner(ctx)
    }
}

#[event]
pub struct OperatorAdded {
    pub key: Pubkey,
}

#[event]
pub struct OperatorRemoved {
    pub key: Pubkey,
}

#[event]
pub struct OwnershipTransferred {
    pub old_owner: Pubkey,
    pub new_owner: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Only the master operator can perform this action")]
    UnauthorizedOwner,
    #[msg("Invalid operator account")]
    InvalidOperator,
    #[msg("New master cannot be the same as current master")]
    SameMaster,
}

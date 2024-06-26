//! Error types

use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
/// Errors of [InterchainTokenServiceError] program.
pub enum InterchainTokenServiceError {
    /// InvalidInstruction
    #[error("invalid instruction")]
    InvalidInstruction,

    /// InvalidSystemAccount
    #[error("invalid system account")]
    InvalidSystemAccount,

    /// InvalidSPLTokenProgram
    #[error("invalid SPL token program")]
    InvalidSPLTokenProgram,

    /// UnsupportedTokenManagerType
    #[error("unsupported token manager type")]
    UnsupportedTokenManagerType,

    /// Unimplemented
    #[error("unimplemented")]
    Unimplemented,

    /// UninitializedITSRootPDA
    #[error("uninitialized ITS root PDA")]
    UninitializedITSRootPDA,

    /// UninitializedMintAccount
    #[error("uninitialized mint account")]
    UninitializedMintAccount,

    /// InvalidMintAccountOwner
    #[error("invalid mint account owner")]
    InvalidMintAccountOwner,

    /// InvalidITSATA
    #[error("invalid interchain token service, associated token account")]
    InvalidITSATA,

    /// UninitializedAssociatedTokenAccount
    #[error("uninitialized associated token account")]
    UninitializedAssociatedTokenAccount,

    /// Emitted when the destination_chain length is 0.
    #[error("untrusted chain")]
    UntrustedChain,
}

impl From<InterchainTokenServiceError> for ProgramError {
    fn from(e: InterchainTokenServiceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

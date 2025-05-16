//! Axelar Gas Service program for the Solana blockchain
#![allow(clippy::little_endian_bytes)]
pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("gasFkyvr4LjK3WwnMGbao3Wzr67F88TmhKmi4ZCXF91");

/// Prefixes to ensure unique PDA addresses
pub mod seed_prefixes {
    /// The seed prefix for deriving Relayer Execution PDA
    pub const RELAYER_EXECUTION: &[u8] = b"relayer-execution";
    /// The seed prefix for deriving Relayer Execution PDA
    pub const SIGNING_PDA: &[u8] = b"signing-pda";
}

/// Event discriminators (prefixes) used to identify logged events related to native gas operations.
pub mod event_prefixes {
    /// Prefix emitted when native gas is paid for a contract call.
    pub const NATIVE_GAS_PAID_FOR_CONTRACT_CALL: &[u8] = b"native gas paid for contract call";
    /// Prefix emitted when native gas is added to an already emtted contract call.
    pub const NATIVE_GAS_ADDED: &[u8] = b"native gas added";
    /// Prefix emitted when native gas is refunded.
    pub const NATIVE_GAS_REFUNDED: &[u8] = b"native gas refunded";

    /// Prefix emitted when SPL token was used to pay for a contract call.
    pub const SPL_PAID_FOR_CONTRACT_CALL: &[u8] = b"spl token paid for contract call";
    /// Prefix emitted when SPL token gas is added to an already emtted contract call.
    pub const SPL_GAS_ADDED: &[u8] = b"spl token gas added";
    /// Prefix emitted when SPL token gas is refunded.
    pub const SPL_GAS_REFUNDED: &[u8] = b"spl token refunded";
}

/// Checks that the provided `program_id` matches the current program’s ID.
///
/// # Errors
///
/// - if the provided `program_id` does not match.
#[inline]
pub fn check_program_account(program_id: Pubkey) -> Result<(), ProgramError> {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives the configuration PDA for this program.
///
/// Given a `program_id`, a `salt` (32-byte array), and an `authority` (`Pubkey`), this function
/// uses [`Pubkey::find_program_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn get_singing_pda(destination_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::SIGNING_PDA],
        destination_address,
    )
}

/// Derives the configuration PDA for this program.
///
/// Given a `program_id`, a `salt` (32-byte array), and an `authority` (`Pubkey`), this function
/// uses [`Pubkey::find_program_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn get_relayer_execution_pda(program_id: &Pubkey, destination_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::RELAYER_EXECUTION, destination_address.as_ref()],
        program_id,
    )
}

/// Derives the configuration PDA for this program.
///
/// Given a `program_id`, a `salt` (32-byte array), and an `authority` (`Pubkey`), this function
/// uses [`Pubkey::check_programm_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn create_relayer_execution_pda(program_id: &Pubkey, destination_address: &Pubkey, bump: u8) -> Result<Pubkey, ProgramError> {
    Pubkey::create_program_address(
        &[seed_prefixes::RELAYER_EXECUTION, destination_address.as_ref(), &[bump]],
        program_id,
    ).map_err(|_| ProgramError::InvalidAccountData)
}

/// Utilities for working with gas service events
pub mod event_utils {

    /// Errors that may occur while parsing a `MessageEvent`.
    #[derive(Debug, thiserror::Error)]
    pub enum EventParseError {
        /// Occurs when a required field is missing in the event data.
        #[error("Missing data: {0}")]
        MissingData(&'static str),

        /// The data is there but it's not of valid format
        #[error("Invalid data: {0}")]
        InvalidData(&'static str),

        /// Occurs when the length of a field does not match the expected length.
        #[error("Invalid length for {field}: expected {expected}, got {actual}")]
        InvalidLength {
            /// the field that we're trying to parse
            field: &'static str,
            /// the desired length
            expected: usize,
            /// the actual length
            actual: usize,
        },

        /// Occurs when a field contains invalid UTF-8 data.
        #[error("Invalid UTF-8 in {field}: {source}")]
        InvalidUtf8 {
            /// the field we're trying to parse
            field: &'static str,
            /// underlying error
            #[source]
            source: std::string::FromUtf8Error,
        },

        /// Generic error for any other parsing issues.
        #[error("Other error: {0}")]
        Other(&'static str),
    }
}

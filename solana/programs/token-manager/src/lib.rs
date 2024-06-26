#![deny(missing_docs)]

//! This program is responsible for managing tokens, such as setting locking
//! token balances, or setting flow limits, for interchain transfers.

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
use std::ops::Deref;

use borsh::{BorshDeserialize, BorshSerialize};
pub use solana_program;
use solana_program::clock::Clock;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;

solana_program::declare_id!("CjPg9dHvYxy6R8HBoYaTLubsZUoSWzXD5GKNbUy6Yz47");

/// Represents a calculated epoch.
/// https://github.com/axelarnetwork/interchain-token-service/blob/main/contracts/utils/FlowLimit.sol#L120
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculatedEpoch(u64);

impl Default for CalculatedEpoch {
    fn default() -> Self {
        Self::new()
    }
}

/// The type of token manager.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum TokenManagerType {
    /// Will simply transfer tokens from a user to itself or vice versa to
    /// initiate/fulfill cross-chain transfers.
    MintBurn = 0,

    /// Works like the one above, but accounts for tokens that have a
    /// fee-on-transfer giving less tokens to be locked than what it actually
    /// transferred.
    MintBurnFrom = 1,

    /// Will burn/mint tokens from/to the user to initiate/fulfill cross-chain
    /// transfers. Tokens used with this kind of TokenManager need to be
    /// properly permissioned to allow for this behaviour.
    LockUnlock = 2,

    /// The same as the one above, but uses burnFrom instead of burn which is
    /// the standard for some tokens and typically requires an approval.
    LockUnlockFee = 3,
}

impl TryFrom<u8> for TokenManagerType {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MintBurn),
            1 => Ok(Self::MintBurnFrom),
            2 => Ok(Self::LockUnlock),
            3 => Ok(Self::LockUnlockFee),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl From<TokenManagerType> for u8 {
    fn from(value: TokenManagerType) -> Self {
        value as u8
    }
}

impl CalculatedEpoch {
    /// Creates a new `CalculatedEpoch` from the current timestamp within a
    /// Solana program.
    pub fn new() -> Self {
        Self::new_with_timestamp(
            Clock::get()
                .expect("Failed to get clock")
                .unix_timestamp
                .try_into()
                .expect("Failed to convert timestamp to u64"),
        )
    }

    /// Creates a new `CalculatedEpoch` from a given timestamp.
    pub fn new_with_timestamp(block_timestamp: u64) -> Self {
        const SIX_HOURS_SEC: u64 = 6 * 60 * 60;
        Self(block_timestamp / SIX_HOURS_SEC)
    }
}

impl Deref for CalculatedEpoch {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Calculates the program-derived address (PDA) for the token manager account.
///
/// This function is a public interface for
/// `get_token_manager_account_and_bump_seed_internal`. It takes the same
/// arguments but does not expose the bump seed in the return value.
///
/// # Arguments
///
/// * `operators_permission_group_pda`: The public key of the operator group's
///   program-derived address (PDA).
/// * `flow_limiter_group_pda`: The public key of the flow limiter group's
///   program-derived address (PDA).
/// * `service_program_pda`: The public key of the service program's
///   program-derived address (PDA).
///
/// # Returns
///
/// This function returns the public key of the token manager account.
pub fn get_token_manager_account(
    operators_permission_group_pda: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
) -> Pubkey {
    get_token_manager_account_and_bump_seed_internal(
        operators_permission_group_pda,
        flow_limiter_group_pda,
        service_program_pda,
        &id(),
    )
    .0
}

/// Calculates the program-derived address (PDA) for the token flow account.
pub fn get_token_flow_account(token_manager_pda: &Pubkey, epoch: CalculatedEpoch) -> Pubkey {
    get_token_flow_account_and_bump_seed_internal(token_manager_pda, epoch, &id()).0
}

/// calculates the program-derived address (PDA) for the token manager account.
pub(crate) fn get_token_manager_account_and_bump_seed_internal(
    operators_permission_group_pda: &Pubkey,
    flow_limiters_permission_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            (operators_permission_group_pda.as_ref()),
            (flow_limiters_permission_group_pda.as_ref()),
            (service_program_pda.as_ref()),
        ],
        program_id,
    )
}

pub(crate) fn get_token_flow_account_and_bump_seed_internal(
    token_manager_pda: &Pubkey,
    epoch: CalculatedEpoch,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[(token_manager_pda.as_ref()), &epoch.to_le_bytes()],
        program_id,
    )
}

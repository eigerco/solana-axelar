//! State structures for token manager program

use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::TokenManagerType;

/// Represents a Token Manager Account in the Solana blockchain.
///
/// This struct is used to manage the flow of tokens in a Solana program. It
/// keeps track of the incoming tokens (`flow_in`), outgoing tokens
/// (`flow_out`), and the maximum allowed tokens that can flow (`flow_limit`).
/// ```
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TokenManagerRootAccount {
    /// The actual token mint that TokenManager is managing
    pub token_mint: Pubkey,
    /// The associated token account that TokenManager has ownership over
    pub associated_token_account: Pubkey,
    /// The total number of tokens that have flowed into the account.
    pub flow_limit: u64,
    /// The type of token manager.
    pub token_manager_type: TokenManagerType,
}

impl Sealed for TokenManagerRootAccount {}
impl Pack for TokenManagerRootAccount {
    const LEN: usize = size_of::<TokenManagerRootAccount>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}

/// Represents Flow In and Flow Out in the account state
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TokenManagerFlowInOutAccount {
    /// The total number of tokens that have flowed into the account.
    pub flow_in: u64,
    /// The total number of tokens that have flowed out of the account.
    pub flow_out: u64,
}

impl Sealed for TokenManagerFlowInOutAccount {}
impl Pack for TokenManagerFlowInOutAccount {
    const LEN: usize = size_of::<TokenManagerFlowInOutAccount>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_root_pda() {
        let token_manager_root_pda = TokenManagerRootAccount {
            token_mint: Pubkey::new_unique(),
            associated_token_account: Pubkey::new_unique(),
            flow_limit: 100,
            token_manager_type: TokenManagerType::LockUnlock,
        };
        let mut packed = vec![0; TokenManagerRootAccount::LEN];
        TokenManagerRootAccount::pack_into_slice(&token_manager_root_pda, &mut packed);
        let unpacked = TokenManagerRootAccount::unpack_from_slice(&packed).unwrap();
        assert_eq!(token_manager_root_pda, unpacked);
    }
}

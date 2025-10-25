#![warn(missing_docs, unreachable_pub)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

//! Simple memo program example for the Axelar Gateway on Solana

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
use program_utils::ensure_single_feature;
pub use solana_program;
use solana_program::pubkey::Pubkey;
use state::Payload;

use crate::state::Storage;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("memPJFxP6H6bjEKpUSJ4KC7C4dKAfNE3xWrTpJBKDwN");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("memdp6koMvx6Bneq1BJvtf7YEKNQDiNmnMFfE6fP691");

#[cfg(feature = "testnet")]
solana_program::declare_id!("memgw1yvm5Q4MVzsTyyz7MdzMUtB1wZC8HeH2ZJABh2");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("mem1111111111111111111111111111111111111111");

/// Derives interchain token service root PDA
pub(crate) fn get_storage_pda_internal(storage_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&storage_id.to_le_bytes()], program_id)
}

/// Derives interchain token service root PDA
pub fn get_storage_pda(storage_id: u64) -> (Pubkey, u8) {
    get_storage_pda_internal(storage_id, &crate::ID)
}

/// Assert counter PDA seeds
fn assert_storage_pda_seeds(storage_id: u64, storage: Storage, storage_pda: &Pubkey) {
    let derived = Pubkey::create_program_address(&[&storage_id.to_le_bytes(), &[storage.bump]], &crate::ID)
        .expect("failed to derive PDA");
    assert_eq!(&derived, storage_pda, "invalid pda for memo counter");
}

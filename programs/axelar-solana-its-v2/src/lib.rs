//! Axelar Interchain Token Service program for the Solana blockchain
#![allow(clippy::little_endian_bytes)]
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

use anchor_lang::prelude::*;
use program_utils::ensure_single_feature;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
declare_id!("itsqybuNsChBo3LgVhCWWnTJVJdoVTUJaodmqQcG6z7");

#[cfg(feature = "stagenet")]
declare_id!("itsediSVCwwKc6UuxfrsEiF8AEuEFk34RFAscPEDEpJ");

#[cfg(feature = "testnet")]
declare_id!("itsZEirFsnRmLejCsRRNZKHqWTzMsKGyYi6Qr962os4");

#[cfg(feature = "mainnet")]
declare_id!("its1111111111111111111111111111111111111111");

#[program]
pub mod axelar_solana_its_v2 {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        chain_name: String,
        its_hub_address: String,
    ) -> Result<()> {
        instructions::initialize::initialize(ctx, chain_name, its_hub_address)
    }

    pub fn set_pause_status(ctx: Context<SetPauseStatus>, paused: bool) -> Result<()> {
        instructions::set_pause_status::set_pause_status(ctx, paused)
    }

    pub fn set_trusted_chain(ctx: Context<SetTrustedChain>, chain_name: String) -> Result<()> {
        instructions::set_trusted_chain::set_trusted_chain(ctx, chain_name)
    }

    pub fn remove_trusted_chain(
        ctx: Context<RemoveTrustedChain>,
        chain_name: String,
    ) -> Result<()> {
        instructions::remove_trusted_chain::remove_trusted_chain(ctx, chain_name)
    }
}

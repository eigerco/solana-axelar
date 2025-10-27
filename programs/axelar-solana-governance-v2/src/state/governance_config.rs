use std::ops::RangeInclusive;

use crate::GovernanceError;
use anchor_lang::prelude::*;

pub type Hash = [u8; 32];
/// The [`solana_program::pubkey::Pubkey`] bytes.
pub type Address = [u8; 32];

pub const VALID_PROPOSAL_DELAY_RANGE: RangeInclusive<u32> = 3600..=86400;

/// Governance configuration type.
#[derive(Debug)]
#[account]
pub struct GovernanceConfig {
    /// The bump for this account.
    pub bump: u8,
    /// The name hash of the governance chain of the remote GMP contract. This
    /// param is used for validating the incoming GMP governance message.
    pub chain_hash: Hash,
    /// The address hash of the remote GMP governance contract. This param
    /// is used for validating the incoming GMP governance message.
    pub address_hash: Hash,
    /// This is the minimum time in seconds from `now()` a proposal can
    /// be executed. If the incoming GMP proposal does not have an ETA
    /// superior to `unix_timestamp` + `this field`, such ETA will be
    /// will be set as new ETA.
    pub minimum_proposal_eta_delay: u32,
    /// The pub key of the operator. This address is able to execute proposals
    /// that were previously scheduled by the Axelar governance infrastructure
    /// via GMP flow regardless of the proposal ETA.
    pub operator: Address,
}

impl anchor_lang::Space for GovernanceConfig {
    const INIT_SPACE: usize =
		1 + // bump
		32 + // chain_hash
		32 + // address_hash
		4 + // minimum_proposal_eta_delay
		32 // operator
		;
}

impl GovernanceConfig {
    pub const SEED_PREFIX: &'static [u8] = b"governance";

    pub fn find_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX], &crate::ID)
    }

    pub fn validate_config(&self) -> Result<()> {
        if !VALID_PROPOSAL_DELAY_RANGE.contains(&self.minimum_proposal_eta_delay) {
            msg!(
                "The minimum proposal ETA delay must be among {} and {} seconds",
                VALID_PROPOSAL_DELAY_RANGE.start(),
                VALID_PROPOSAL_DELAY_RANGE.end()
            );
            return err!(GovernanceError::InvalidArgument);
        }
        Ok(())
    }

    pub fn update(&mut self, mut update: GovernanceConfigUpdate) -> Result<()> {
        if let Some(new_chain_hash) = update.chain_hash.take() {
            self.chain_hash = new_chain_hash;
        }

        if let Some(new_address_hash) = update.address_hash.take() {
            self.address_hash = new_address_hash;
        }

        if let Some(new_minimum_proposal_eta_delay) = update.minimum_proposal_eta_delay.take() {
            self.minimum_proposal_eta_delay = new_minimum_proposal_eta_delay;
        }
        self.validate_config()
    }
}

/// Parameters for initializing a new governance config.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct GovernanceConfigInit {
    pub chain_hash: Hash,
    pub address_hash: Hash,
    pub minimum_proposal_eta_delay: u32,
    pub operator: Address,
}

impl GovernanceConfigInit {
    /// Creates a new governance program config.
    #[must_use]
    pub const fn new(
        chain_hash: Hash,
        address_hash: Hash,
        minimum_proposal_eta_delay: u32,
        operator: Address,
    ) -> Self {
        Self {
            chain_hash,
            address_hash,
            minimum_proposal_eta_delay,
            operator,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GovernanceConfigUpdate {
    pub chain_hash: Option<Hash>,
    pub address_hash: Option<Hash>,
    pub minimum_proposal_eta_delay: Option<u32>,
}

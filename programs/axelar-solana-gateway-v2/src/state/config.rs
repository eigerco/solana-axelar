use crate::{u256::U256, GatewayError};
use anchor_lang::prelude::*;

/// Timestamp alias for when the last signer rotation happened
pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type VerifierSetEpoch = U256;

#[account(zero_copy)]
#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::pub_underscore_fields)]
pub struct GatewayConfig {
    /// current epoch points to the latest signer set hash
    pub current_epoch: VerifierSetEpoch,
    /// how many n epochs do we consider valid
    pub previous_verifier_set_retention: VerifierSetEpoch,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// timestamp tracking of when the previous rotation happened
    pub last_rotation_timestamp: Timestamp,
    /// The gateway operator.
    pub operator: Pubkey,
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// The canonical bump for this account.
    pub bump: u8,
    /// padding for bump
    pub _padding: [u8; 7],
}

impl GatewayConfig {
    pub const SEED_PREFIX: &'static [u8] = b"gateway";

    pub fn find_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX], &crate::ID)
    }

    pub fn assert_valid_epoch(&self, epoch: U256) -> Result<()> {
        let current_epoch = self.current_epoch;
        let elapsed = current_epoch
            .checked_sub(epoch)
            .ok_or(GatewayError::EpochCalculationOverflow)?;

        if elapsed >= self.previous_verifier_set_retention {
            return err!(GatewayError::VerifierSetTooOld);
        }
        Ok(())
    }
}

pub type VerifierSetHash = [u8; 32];

/// Represents an initial verifier set with its hash and PDA
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct InitialVerifierSet {
    /// The hash of the verifier set
    pub hash: VerifierSetHash,
    /// The PDA for the verifier set tracker
    pub pda: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeConfigParams {
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// initial verifier set
    pub initial_verifier_set: InitialVerifierSet,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_verifier_retention: VerifierSetEpoch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_solana_gateway::state::GatewayConfig as GatewayConfigV1;
    use bytemuck::bytes_of;

    #[test]
    fn test_v1_compat() {
        assert_eq!(
            std::mem::size_of::<GatewayConfig>(),
            std::mem::size_of::<GatewayConfigV1>()
        );

        // Create matching instances
        let current_epoch_v1 = axelar_message_primitives::U256::from(42u64);
        let previous_verifier_set_retention_v1 = axelar_message_primitives::U256::from(10u64);

        let current_epoch = U256::from(42u64);
        let previous_verifier_set_retention = U256::from(10u64);
        let minimum_rotation_delay = 86400u64; // 1 day
        let last_rotation_timestamp = 1_234_567_890_u64;
        let operator = Pubkey::new_unique();
        let domain_separator = [0x42u8; 32];
        let bump = 255u8;

        let v1_config = GatewayConfigV1::new(
            current_epoch_v1,
            previous_verifier_set_retention_v1,
            minimum_rotation_delay,
            last_rotation_timestamp,
            operator,
            domain_separator,
            bump,
        );

        let v2_config = GatewayConfig {
            current_epoch,
            previous_verifier_set_retention,
            minimum_rotation_delay,
            last_rotation_timestamp,
            operator,
            domain_separator,
            bump,
            _padding: [0; 7],
        };

        // Compare byte representations
        assert_eq!(bytes_of(&v1_config), bytes_of(&v2_config));
    }
}

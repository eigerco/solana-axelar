use anchor_lang::prelude::*;

// TODO(v2) check sizes
pub const ITS_HUB_ADDRESS_MAX_LEN: usize = 45;
pub const DEFAULT_RESERVED_LEN_TRUSTED_CHAINS: usize = 30;
pub const MAX_CHAIN_NAME_LEN: usize = 30;

#[account]
#[derive(InitSpace, PartialEq, Eq, Debug)]
pub struct InterchainTokenService {
    /// The address of the Axelar ITS Hub contract.
    #[max_len(ITS_HUB_ADDRESS_MAX_LEN)]
    pub its_hub_address: String,

    /// Name of the chain ITS is running on.
    #[max_len(MAX_CHAIN_NAME_LEN)]
    pub chain_name: String,

    /// Whether the ITS is paused.
    pub paused: bool,

    /// Trusted chains
    // TODO(v2) maybe use HashSet or light hash set
    // https://github.com/Lightprotocol/light-protocol/blob/light-hash-set-v2.0.0/program-libs/hash-set/src/lib.rs
    #[max_len(DEFAULT_RESERVED_LEN_TRUSTED_CHAINS, MAX_CHAIN_NAME_LEN)]
    pub trusted_chains: Vec<String>,

    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl InterchainTokenService {
    pub const SEED_PREFIX: &'static [u8] = b"interchain-token-service";

    pub fn space(trusted_chains_len: usize) -> usize {
        InterchainTokenService::DISCRIMINATOR.len() + // Anchor account discriminator
		4 + ITS_HUB_ADDRESS_MAX_LEN + // its_hub_address
		4 + MAX_CHAIN_NAME_LEN + // chain_name
		1 + // paused (bool)
		4 +
			// number of trusted chains, min to reserve space
			(trusted_chains_len.max(DEFAULT_RESERVED_LEN_TRUSTED_CHAINS)
			// trusted_chains (Vec<String> with max chain name length)
			* (4 + MAX_CHAIN_NAME_LEN)) +
		1 // bump (u8)
    }

    /// Create a new `InterchainTokenService` instance.
    #[must_use]
    pub fn new(bump: u8, chain_name: String, its_hub_address: String) -> Self {
        Self {
            its_hub_address,
            chain_name,
            paused: false,
            trusted_chains: Vec::new(),
            bump,
        }
    }

    /// Pauses the Interchain Token Service.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpauses the Interchain Token Service.
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    pub fn is_trusted_chain(&self, chain_name: String) -> bool {
        self.trusted_chains.iter().any(|chain| *chain == chain_name)
    }

    /// Add a chain as trusted
    pub fn add_trusted_chain(&mut self, chain_name: String) {
        // Only add if not already present to avoid duplicates
        if !self.trusted_chains.iter().any(|chain| *chain == chain_name) {
            self.trusted_chains.push(chain_name);
        }
    }

    /// Remove a chain from trusted
    pub fn remove_trusted_chain(&mut self, chain_name: String) {
        self.trusted_chains.retain(|chain| *chain != chain_name);
    }
}

#[cfg(test)]
#[allow(clippy::str_to_string)]
mod tests {
    use super::*;
    use anchor_lang::AnchorSerialize;

    #[test]
    fn test_space_function_matches_actual_size() {
        // Test with empty trusted chains
        let its_empty = InterchainTokenService {
            its_hub_address: "test".to_string(),
            chain_name: "solana".to_string(),
            paused: false,
            trusted_chains: vec![],
            bump: 1,
        };

        let serialized = its_empty.try_to_vec().expect("Failed to serialize");
        let calculated_space = InterchainTokenService::space(0);

        assert!(
            calculated_space >= serialized.len(),
            "Space function should account for at least the actual size"
        );
    }

    #[test]
    fn test_space_function_with_few_chains() {
        let its = InterchainTokenService {
            its_hub_address: "test".to_string(),
            chain_name: "solana".to_string(),
            paused: false,
            trusted_chains: vec![
                "ethereum".to_string(),
                "polygon".to_string(),
                "avalanche".to_string(),
            ],
            bump: 1,
        };

        let serialized = its.try_to_vec().expect("Failed to serialize");
        let calculated_space = InterchainTokenService::space(3);

        assert!(
            calculated_space >= serialized.len(),
            "Space function should account for at least the actual size"
        );
    }

    #[test]
    fn test_space_function_with_many_chains() {
        let trusted_chains: Vec<String> = (0..40).map(|i| format!("chain_{i}")).collect();

        let its = InterchainTokenService {
            its_hub_address: "test".to_string(),
            chain_name: "solana".to_string(),
            paused: false,
            trusted_chains,
            bump: 1,
        };

        let serialized = its.try_to_vec().expect("Failed to serialize");
        let calculated_space = InterchainTokenService::space(40);

        assert!(
            calculated_space >= serialized.len(),
            "Space function should account for at least the actual size"
        );
    }

    #[test]
    fn test_space_function_with_max_length_data() {
        let max_hub_address = "x".repeat(ITS_HUB_ADDRESS_MAX_LEN);
        let max_chain_name = "y".repeat(MAX_CHAIN_NAME_LEN);
        let max_trusted_chains: Vec<String> = (0..10)
            .map(|i| format!("{}{}", "z".repeat(MAX_CHAIN_NAME_LEN - 1), i))
            .collect();

        let its = InterchainTokenService {
            its_hub_address: max_hub_address,
            chain_name: max_chain_name,
            paused: true,
            trusted_chains: max_trusted_chains,
            bump: 255,
        };

        let serialized = its.try_to_vec().expect("Failed to serialize");
        let calculated_space = InterchainTokenService::space(10);

        assert!(
            calculated_space >= serialized.len(),
            "Space function should account for at least the actual size"
        );
    }
}

//! Utilities for working with the Axelar gas service

use crate::base::TestFixture;

use solana_program_test::tokio;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};


/// Utility structure for keeping gas service related state
pub struct RelayerDiscoveryUtils {
    /// upgrade authority of the program
    pub upgrade_authority: Keypair,
}

impl TestFixture {
    /// Deploy the gas service program and construct a pre-emptive
    pub async fn deploy_relayer_discovery(&mut self) -> RelayerDiscoveryUtils {
        // deploy gas service
        let gas_service_bytecode =
            tokio::fs::read("../../target/deploy/axelar_solana_relayer_discovery.so")
                .await
                .unwrap();

        // Generate a new keypair for the upgrade authority
        let upgrade_authority = Keypair::new();

        self.register_upgradeable_program(
            &gas_service_bytecode,
            &upgrade_authority.pubkey(),
            &axelar_solana_relayer_discovery::id(),
        )
        .await;
        
        RelayerDiscoveryUtils {
            upgrade_authority,
        }
    }
}
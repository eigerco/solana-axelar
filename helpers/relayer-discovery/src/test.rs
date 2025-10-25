use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway_test_fixtures::{gateway::random_message, SolanaAxelarIntegrationMetadata};
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::state::incoming_message::command_id;
use borsh::{to_vec, BorshDeserialize};
use ethers_core::utils::keccak256;
use crate::{find_transaction_pda, RelayerDiscovery};
use crate::structs::RelayerTransaction;
use solana_sdk::signature::{Keypair, Signer};

/// Approve a message with the gateway and create a `RelayerDiscovery` associated with it.
pub async fn create_relayer_discovery_for_testing(
    solana_chain: &mut SolanaAxelarIntegrationMetadata,
    message: Message,
    payload: Vec<u8>,
) -> RelayerDiscovery {
    let messages = vec![
        message.clone(),
    ];
    // Action: "Relayer" calls Gateway to approve messages
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &messages)
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (message_pda, ..) = get_incoming_message_pda(&command_id(
        &message.cc_id.chain,
        &message.cc_id.id,
    ));

    RelayerDiscovery {
        message,
        message_pda,
        payload,
        payload_pda: None,
        payers: vec![],
    }
}
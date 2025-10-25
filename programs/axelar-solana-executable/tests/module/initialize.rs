use axelar_solana_executable::instruction::{get_transaction, AxelarExecutableInstruction};
use axelar_solana_executable::state::Payload;
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::state::incoming_message::command_id;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use borsh::{to_vec, BorshDeserialize};
use ethers_core::utils::keccak256;
use relayer_discovery::test::create_relayer_discovery_for_testing;
use relayer_discovery::{find_transaction_pda, ConvertedTransaction, RelayerDiscovery};
use relayer_discovery::structs::{RelayerData, RelayerInstruction, RelayerTransaction};
use solana_program_test::tokio;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signer};

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
async fn test_initialize() {
    // Setup
    let mut solana_chain = program_test().await;
    // Action
    let initialize = axelar_solana_executable::instruction::initialize(
        &solana_chain.fixture.payer.pubkey().clone(),
    )
    .unwrap();

    solana_chain.send_tx(&[initialize]).await.unwrap();


    let (transaction_pda, _) = find_transaction_pda(&axelar_solana_executable::ID);
    // Assert
    let transaction_pda = solana_chain
        .fixture
        .get_account(&transaction_pda, &axelar_solana_executable::id())
        .await;
    let transaction = RelayerTransaction::try_from_slice(&transaction_pda.data).unwrap();

    // Test scoped constants
    let random_account_used_by_ix = Keypair::new();
    let destination_program_id = axelar_solana_executable::id();
    let memo_string = String::from("🐪🐪🐪🐪");
    let storage_id = 12;

    let payload = Payload {
        storage_id,
        value: memo_string,
    };
    let payload_bytes = to_vec(&payload).unwrap();
    let payload_hash = keccak256(&payload_bytes);
    let mut message = random_message();
    message.payload_hash = payload_hash;
    message.destination_address = axelar_solana_executable::id().to_string();

    let relayer_discovery = create_relayer_discovery_for_testing(&mut solana_chain, message, payload_bytes.clone()).await;
    let tx = RelayerTransaction::Discovery(RelayerInstruction { 
        program_id: axelar_solana_executable::id(), 
        accounts: vec![], 
        data: vec![
            RelayerData::Bytes(vec![1, 2, 3, 4]),
            RelayerData::Payload,
        ], 
    });
    dbg!(&tx);
    dbg!(&to_vec(&tx).unwrap());
    assert_eq!(relayer_discovery.convert_transaction(&transaction). unwrap(), ConvertedTransaction::Discovery(get_transaction(payload_bytes).unwrap()));
}

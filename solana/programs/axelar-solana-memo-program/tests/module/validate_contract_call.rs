use axelar_executable::axelar_message_primitives::command::DecodedCommand;
use axelar_executable::axelar_message_primitives::{DestinationProgramId, EncodingScheme};
use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::instruction::from_axelar_to_solana::build_memo;
use gateway::state::GatewayApprovedCommand;
use itertools::Either;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::{create_signer_with_weight, TestSigner};
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[rstest::rstest]
#[case(EncodingScheme::Borsh)]
#[case(EncodingScheme::AbiEncoding)]
#[tokio::test]
async fn test_successful_validate_contract_call(#[case] encoding_scheme: EncodingScheme) {
    // Setup
    let (mut solana_chain, gateway_root_pda, solana_operators, counter_pda) = solana_setup().await;

    // Test scoped constants
    let random_account_used_by_ix = Keypair::new();
    let destination_program_id = DestinationProgramId(axelar_solana_memo_program::id());
    let memo_string = "🐪🐪🐪🐪";

    // Create 2 messages: one we're going to execute and one we're not
    let message_payload = build_memo(
        memo_string.as_bytes(),
        &counter_pda,
        &[&random_account_used_by_ix.pubkey()],
        encoding_scheme,
    );
    let message_to_execute =
        custom_message(destination_program_id, message_payload.clone()).unwrap();
    let other_message_in_the_batch =
        custom_message(destination_program_id, message_payload.clone()).unwrap();
    let messages = vec![
        Either::Left(message_to_execute.clone()),
        Either::Left(other_message_in_the_batch),
    ];

    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, gateway_execute_data, _) = solana_chain
        .fully_approve_messages(&gateway_root_pda, &messages, &solana_operators)
        .await;

    // Action: set message status as executed by calling the destination program
    let tx = solana_chain
        .call_execute_on_axelar_executable(
            &gateway_execute_data.command_batch.commands[0],
            &message_payload,
            &gateway_approved_command_pdas[0],
            gateway_root_pda,
        )
        .await;
    assert!(tx.result.is_ok(), "transaction failed");

    // Assert
    // First message should be executed
    let gateway_approved_message = solana_chain
        .get_account::<GatewayApprovedCommand>(&gateway_approved_command_pdas[0], &gateway::id())
        .await;
    assert!(gateway_approved_message.is_command_executed());

    // The second message is still in Approved status
    let gateway_approved_message = solana_chain
        .get_account::<GatewayApprovedCommand>(&gateway_approved_command_pdas[1], &gateway::id())
        .await;
    assert!(gateway_approved_message.is_contract_call_approved());

    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains(&format!(
            "{:?}-{}-{}",
            random_account_used_by_ix.pubkey(),
            false,
            false
        ))),
        "expected memo not found in logs"
    );

    // The counter should have been incremented
    let counter_account = solana_chain
        .get_account::<axelar_solana_memo_program::state::Counter>(
            &counter_pda,
            &axelar_solana_memo_program::id(),
        )
        .await;
    assert_eq!(counter_account.counter, 1);
}

async fn solana_setup() -> (
    TestFixture,
    solana_sdk::pubkey::Pubkey,
    Vec<TestSigner>,
    solana_sdk::pubkey::Pubkey,
) {
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let (counter_pda, counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&gateway_root_pda);
    fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &fixture.payer.pubkey(),
            &gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    (fixture, gateway_root_pda, operators, counter_pda)
}

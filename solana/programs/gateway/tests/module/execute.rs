use axelar_message_primitives::command::{DecodedCommand, U256};
use axelar_message_primitives::DestinationProgramId;
use cosmwasm_std::Uint256;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::state::{
    GatewayApprovedCommand, GatewayCommandStatus, GatewayConfig, GatewayExecuteData,
};
use itertools::{Either, Itertools};
use multisig::key::Signature;
use multisig::worker_set::WorkerSet;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use test_fixtures::axelar_message::{custom_message, new_worker_set, WorkerSetExt};
use test_fixtures::execute_data::{
    self, create_command_batch, create_signer_with_weight, sign_batch, TestSigner,
};

use crate::{example_payload, setup_initialised_gateway};

#[tokio::test]
async fn successfully_process_execute_when_there_are_no_commands() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let messages = [];
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok())
}

/// successfully process execute when there are 3 validate contract call
/// commands - emits message approved events
#[tokio::test]
async fn successfully_process_execute_when_there_are_3_validate_contract_call_commands() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - events get emitted
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs =
        get_gateway_events_from_execute_data(&execute_data.command_batch.commands);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    for gateway_approved_command_pda in gateway_approved_command_pdas.iter() {
        let approved_commmand =
            get_approved_commmand(&mut fixture, gateway_approved_command_pda).await;
        assert!(approved_commmand.is_contract_call_approved());
    }
}

/// successfully process execute when there is 1 transfer operatorship commands
#[tokio::test]
async fn successfully_process_execute_when_there_is_1_transfer_operatorship_command() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let (new_worker_set, new_signers) = create_worker_set(&[500_u128, 200_u128], 700_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs =
        get_gateway_events_from_execute_data(&execute_data.command_batch.commands);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    for gateway_approved_command_pda in gateway_approved_command_pdas.iter() {
        let approved_commmand =
            get_approved_commmand(&mut fixture, gateway_approved_command_pda).await;
        assert!(approved_commmand.is_command_executed());
    }

    // - operators have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(2_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .operator_hash_for_epoch(&new_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );

    // - test that both operator sets can sign new messages
    for operator_set in [new_signers, operators] {
        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        fixture
            .fully_approve_messages(
                &gateway_root_pda,
                &[Either::Left(
                    custom_message(destination_program_id, example_payload()).unwrap(),
                )],
                &operator_set,
            )
            .await;
    }
}

/// successfully process execute when there is 1 transfer operatorship and 3
/// validate contract call commands
#[tokio::test]
async fn successfully_process_execute_when_there_is_1_transfer_operatorship_command_and_3_validate_contract_call_commands(
) {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let (new_worker_set, _) = create_worker_set(&[500_u128, 200_u128], 700_u128);
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
        Either::Right(new_worker_set.clone()),
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
    ];
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - events emitted
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs =
        get_gateway_events_from_execute_data(&execute_data.command_batch.commands);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    for gateway_approved_command_pda in gateway_approved_command_pdas.iter() {
        let approved_commmand =
            get_approved_commmand(&mut fixture, gateway_approved_command_pda).await;
        assert!(approved_commmand.is_command_executed());
    }

    // - operators updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(2_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .operator_hash_for_epoch(&new_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// successfully process execute when there are 3 transfer operatorship commands
/// - only the first one should be executed
#[tokio::test]
async fn successfully_process_execute_when_there_are_3_transfer_operatorship_commands() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;

    let (new_worker_set_one, _) = create_worker_set(&[11_u128, 22_u128], 10_u128);
    let (new_worker_set_two, _) = create_worker_set(&[33_u128, 44_u128], 10_u128);
    let (new_worker_set_three, _) = create_worker_set(&[55_u128, 66_u128], 10_u128);

    let messages = [
        new_worker_set_one.clone(),
        new_worker_set_two.clone(),
        new_worker_set_three.clone(),
    ]
    .map(Either::Right);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - events emitted
    let emitted_events = get_gateway_events(&tx);
    assert_eq!(
        emitted_events.len(),
        1,
        "only a single event expected (1 transfer ops executed, rest ignored)"
    );
    let expected_approved_command_logs =
        gmp_gateway::events::GatewayEvent::from(execute_data.command_batch.commands[0].clone());
    assert_eq!(emitted_events[0], expected_approved_command_logs);
    // - all commands get updated
    let approved_commmand =
        get_approved_commmand(&mut fixture, &gateway_approved_command_pdas[0]).await;
    assert!(
        approved_commmand.is_command_executed(),
        "the first transfer command is expected to be executed"
    );
    for gateway_approved_command_pda in gateway_approved_command_pdas.iter().skip(1) {
        let approved_commmand =
            get_approved_commmand(&mut fixture, gateway_approved_command_pda).await;
        assert_eq!(
            approved_commmand.status(),
            &GatewayCommandStatus::TransferOperatorship(
                gmp_gateway::state::TransferOperatorship::Pending
            ),
            "subsequet transfer ops must remain ignored and unaltered"
        );
    }

    // - operators updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(2_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .operator_hash_for_epoch(&new_epoch)
            .unwrap(),
        &new_worker_set_one.hash_solana_way(),
    );
}

/// calling the same execute flow multiple times with the same execute data will
/// not "approve" the command twice.
#[tokio::test]
async fn successfully_consumes_repating_commands_idempotency_same_batch() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    assert!(
        emitted_events.is_empty(),
        "no events should be emitted when processing duplicate commants"
    );
}

/// if a given command is a part of another batch and it's been executed, it
/// should be ignored in subsequent batches if its present in those.
#[tokio::test]
async fn successfully_consumes_repating_commands_idempotency_unique_batches() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Action
    // - we create a new batch with the old command + a new uniqu command
    // NOTE: we need to add a new command because otherwise the `execute data` pda
    // will be the same.
    let messages = [
        messages[0].clone().left().unwrap().clone(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pda_new = fixture
        .init_pending_gateway_commands(
            &gateway_root_pda,
            &[execute_data.command_batch.commands[1].clone()],
        )
        .await;
    let gateway_approved_command_pda_new = gateway_approved_command_pda_new[0];
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &[
                gateway_approved_command_pdas[0],
                gateway_approved_command_pda_new,
            ],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    assert_eq!(
        emitted_events.len(),
        1,
        "only a single event shold be emitted (first command in the batch is ignored)"
    );
}

/// fail if if root config has no operators
#[tokio::test]
async fn fail_if_gateway_config_has_no_operators_signed_by_unknown_operator_set() {
    // Setup
    let (mut fixture, quorum, _operators, gateway_root_pda) =
        setup_initialised_gateway(&[], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (_new_worker_set, operators) = create_worker_set(&[11_u128, 22_u128], 10_u128);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if if root config has no operators and there are no signatures in the
/// execute data
#[tokio::test]
async fn fail_if_gateway_config_has_no_operators_signed_by_empty_set() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    assert!(execute_data.proof.signatures.is_empty());
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// fail if root config not initialised
#[tokio::test]
async fn fail_if_root_config_not_initialised() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let messages = [].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let gateway_root_pda = Pubkey::new_unique();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("insufficient funds") }));
}

/// fail if execute data not initialized
#[tokio::test]
async fn fail_if_execute_data_not_initialised() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let messages = [].map(Either::Left);
    let (_execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let execute_data_pda = Pubkey::new_unique();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("insufficient funds") }));
}

/// fail if invalid account for gateway passed (e.g. initialized command)
#[tokio::test]
async fn fail_if_invalid_account_for_gateway() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_approved_command_pdas[0], // should be gateway_root_pda
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("failed to deserialize account") }));
}

/// fail if invalid account for execute data passed (e.g. initialized command)
#[tokio::test]
async fn fail_if_invalid_account_for_execute_data() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (_execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &gateway_approved_command_pdas[0], // should be execute_data_pda
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Failed to serialize or deserialize account data") }));
}

/// fail if epoch for operators was not found (inside `validate_proof`)
#[tokio::test]
async fn fail_if_epoch_for_operators_was_not_found() {
    // Setup
    let (_unregistered_worker_set, unregistered_worker_set_signers) =
        create_worker_set(&[55_u128, 66_u128], 10_u128);
    let (mut fixture, quorum, _operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &messages,
            &unregistered_worker_set_signers,
            quorum,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if operator epoch is older than 16 epochs away (inside
/// `validate_proof`)
#[tokio::test]
async fn fail_if_operator_epoch_is_older_than_16() {
    use itertools::*;
    // Setup
    let (mut fixture, _quorum, initial_operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let initial_worker_set = new_worker_set(&initial_operators, 0, Uint256::from_u128(33));
    let intial_worker_set = [(initial_worker_set.clone(), initial_operators.clone())];
    let new_worker_sets = (1..=16)
        .map(|x| create_worker_set(&[55_u128, x], 55_u128 + x))
        .collect::<Vec<_>>();

    for (idx, ((_current_worker_set, current_worker_set_signers), (new_worker_set, _))) in
        (intial_worker_set.iter().chain(new_worker_sets.iter()))
            .tuple_windows::<(_, _)>()
            .enumerate()
    {
        let new_epoch = U256::from(idx as u8 + 1_u8);
        let root_pda_data = fixture
            .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
            .await;
        assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
        let messages = [new_worker_set.clone()].map(Either::Right);
        fixture
            .fully_approve_messages(&gateway_root_pda, &messages, current_worker_set_signers)
            .await;
    }

    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(17_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(root_pda_data.auth_weighted.operators().len(), 16);
    // Action
    // we can use any of the 16 operators to sign messages
    for (_, operator_set) in new_worker_sets.iter() {
        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        fixture
            .fully_approve_messages(
                &gateway_root_pda,
                &[Either::Left(
                    custom_message(destination_program_id, example_payload()).unwrap(),
                )],
                operator_set,
            )
            .await;
    }

    // we cannot use the first operator set anymore
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(
            &gateway_root_pda,
            &[Either::Left(
                custom_message(destination_program_id, example_payload()).unwrap(),
            )],
            &initial_operators,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if signatures cannot be recovered (inside `validate_signatures`
/// ProofError::Secp256k1RecoverError)
#[tokio::test]
async fn fail_if_invalid_signatures() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);

    let command_batch = create_command_batch(&messages).unwrap();
    let signatures = {
        // intentionally mangle the signature so it cannot be recovered
        let mut signatures = sign_batch(&command_batch, &operators).unwrap();
        signatures.iter_mut().for_each(|x| {
            x.as_mut().map(|x| {
                let fake_signature = vec![3u8; 65];
                let fake_signature = cosmwasm_std::HexBinary::from(fake_signature.as_slice());
                let fake_signature = Signature::EcdsaRecoverable(
                    multisig::key::Recoverable::try_from(fake_signature).unwrap(),
                );
                *x = fake_signature;

                x
            });
        });
        signatures
    };
    let encoded_message =
        execute_data::encode(&command_batch, operators.to_vec(), signatures, quorum).unwrap();
    let execute_data =
        GatewayExecuteData::new(encoded_message.as_ref(), &gateway_root_pda).unwrap();
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &encoded_message, &execute_data)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(Secp256k1RecoverError(InvalidSignature))") }));
}

/// fail if invalid operators signed the command batch (inside
/// `validate_signatures` ProofError::LowSignatureWeight)
#[tokio::test]
async fn fail_if_invalid_operators_signed_command_batch() {
    // Setup
    let unregistered_worker_set_signer = create_worker_set(&[66_u128], 10_u128).1[0].clone();
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_operators = vec![unregistered_worker_set_signer];
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// fail if small subset operators signed the command batch (inside
/// `validate_signatures` ProofError::LowSignatureWeight)
#[tokio::test]
async fn fail_if_subset_without_expected_weight_signed_batch() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_operators = vec![operators[0].clone(), operators[1].clone()]; // subset of operators
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// succeed if the larger (by wight) subset of operators signed the command
/// batch
#[tokio::test]
async fn succeed_if_majority_of_subset_without_expected_weight_signed_batch() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], Some(150)).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_operators = vec![operators[2].clone()]; // subset of operators
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
}

#[tokio::test]
async fn fail_if_signed_commands_differ_from_the_execute_ones() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let messages_to_sign =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages_to_sign,
        &operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

#[tokio::test]
async fn fail_if_quorum_differs_between_registered_and_signed() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &operators,
        &operators,
        quorum + 1, // quorum is different
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// disallow operatorship transfer if any other operator besides the most recent
/// epoch signed the proof
#[tokio::test]
async fn ignore_transfer_ops_call_if_old_ops_set_initiates_it() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _new_signers) = create_worker_set(&[500_u128, 200_u128], 700_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);
    fixture
        .fully_approve_messages(&gateway_root_pda, &messages, &operators)
        .await;

    // Action - the transfer ops gets ignored because we use `operators`
    let (newer_worker_set, _newer_signers) = create_worker_set(&[444_u128, 555_u128], 333_u128);
    let messages = [newer_worker_set.clone()].map(Either::Right);
    fixture
        .fully_approve_messages(&gateway_root_pda, &messages, &operators)
        .await;

    // Assert
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(2_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&new_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// fail if command len does not match provided account iter len
#[tokio::test]
async fn fail_if_command_len_does_not_match_provided_account_iter_len() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            // we provide only 1 command pda, but there are 3 registered pdas
            &gateway_approved_command_pdas.as_slice()[..1],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        msg.contains("Mismatch between the number of commands and the number of accounts")
    }));
}

/// fail if command was not intialized
#[tokio::test]
async fn fail_if_command_was_not_initialised() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);

    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;

    // Action
    // none of the pdas are initialized
    let gateway_approved_command_pdas = execute_data
        .command_batch
        .commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(&gateway_root_pda, command);
            gateway_approved_message_pda
        })
        .collect_vec();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        // note: error message is not very informative
        msg.contains("insufficient funds for instruction")
    }));
}

#[tokio::test]
async fn fail_if_order_of_commands_is_not_the_same_as_order_of_accounts() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);

    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;

    // Action
    let mut gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    gateway_approved_command_pdas.reverse();

    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
}

/// `transfer_operatorship` is ignored if new operator len is 0 (tx succeeds)
#[tokio::test]
async fn ignore_transfer_ops_if_new_ops_len_is_zero() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&([] as [u128; 0]), 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);
    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;

    // Action
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if new operators are not sorted (tx
/// succeeds)
#[tokio::test]
#[ignore = "cannot implement this without changing the bcs encoding of the `TransferOperatorship` command"]
async fn ignore_transfer_ops_if_new_ops_are_not_sorted() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[555_u128, 678_u128], 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);

    let (mut execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    // reverse the operators
    let decoded_command = execute_data.command_batch.commands.get_mut(0).unwrap();
    if let DecodedCommand::TransferOperatorship(ops) = decoded_command {
        ops.operators.reverse();
    }

    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            // issue: updating the `execute_data` does not update the `gateway_execute_data_raw`
            // which is what we actually use when encoding the data.
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if operator len does not match weigths
/// len (tx succeeds)
#[tokio::test]
#[ignore = "cannot implement this without changing the bcs encoding of the `TransferOperatorship` command"]
async fn ignore_transfer_ops_if_len_does_not_match_weigh_len() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[555_u128, 678_u128], 10_u128);

    let messages = [new_worker_set.clone()].map(Either::Right);

    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    // todo: update the len of operators or weights
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            // issue: updating the `execute_data` does not update the `gateway_execute_data_raw`
            // which is what we actually use when encoding the data.
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// transfer_operatorship` is ignored if total weights sum exceed u256 max (tx
/// succeeds)
#[tokio::test]
#[ignore = "cannot test because the bcs encoding transforms the u256 to a u128 and fails before we actually get to the on-chain logic"]
async fn ignore_transfer_ops_if_total_weight_sum_exceeds_u256() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[Uint256::MAX, Uint256::MAX], 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);

    // Action
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(&gateway_root_pda, &messages, &operators)
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if total weights == 0 (tx succeeds)
#[tokio::test]
#[ignore = "cannot test because the bcs encoding transforms the u256 to a u128 and fails before we actually get to the on-chain logic"]
async fn ignore_transfer_ops_if_total_weight_sum_is_zero() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) =
        create_worker_set(&[Uint256::zero(), Uint256::zero()], 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);

    // Action
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(&gateway_root_pda, &messages, &operators)
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if total weight is smaller than new
/// command weight quorum (tx succeeds)
#[tokio::test]
async fn ignore_transfer_ops_if_total_weight_is_smaller_than_quorum() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[Uint256::one(), Uint256::one()], 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);

    // Action
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(&gateway_root_pda, &messages, &operators)
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_eq!(gateway.auth_weighted.operators().len(), 1);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

fn get_gateway_events_from_execute_data(
    commands: &[axelar_message_primitives::command::DecodedCommand],
) -> Vec<GatewayEvent<'static>> {
    commands
        .iter()
        .cloned()
        .map(gmp_gateway::events::GatewayEvent::from)
        .collect::<Vec<_>>()
}

fn get_gateway_events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<GatewayEvent<'static>> {
    tx.metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .collect::<Vec<_>>()
}

async fn get_approved_commmand(
    fixture: &mut test_fixtures::test_setup::TestFixture,
    gateway_approved_command_pda: &Pubkey,
) -> GatewayApprovedCommand {
    fixture
        .get_account::<gmp_gateway::state::GatewayApprovedCommand>(
            gateway_approved_command_pda,
            &gmp_gateway::ID,
        )
        .await
}

fn create_worker_set(
    weights: &[impl Into<Uint256> + Copy],
    threshold: impl Into<Uint256>,
) -> (multisig::worker_set::WorkerSet, Vec<TestSigner>) {
    let new_operators = weights
        .iter()
        .map(|weight| {
            create_signer_with_weight({
                let weight: Uint256 = (*weight).into();
                weight
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let new_worker_set = new_worker_set(&new_operators, 0, threshold.into());
    (new_worker_set, new_operators)
}

pub fn prepare_questionable_execute_data(
    messages_for_signing: &[Either<connection_router::Message, WorkerSet>],
    messages_for_execute_data: &[Either<connection_router::Message, WorkerSet>],
    signers_for_signatures: &[TestSigner],
    signers_in_the_execute_data: &[TestSigner],
    quorum: u128,
    gateway_root_pda: &Pubkey,
) -> (GatewayExecuteData, Vec<u8>) {
    let command_batch_for_signing = create_command_batch(messages_for_signing).unwrap();
    let command_batch_for_execute_data = create_command_batch(messages_for_execute_data).unwrap();
    let signatures = sign_batch(&command_batch_for_signing, signers_for_signatures).unwrap();
    let encoded_message = execute_data::encode(
        &command_batch_for_execute_data,
        signers_in_the_execute_data.to_vec(),
        signatures,
        quorum,
    )
    .unwrap();
    let execute_data = GatewayExecuteData::new(encoded_message.as_ref(), gateway_root_pda).unwrap();
    (execute_data, encoded_message)
}

use axelar_rkyv_encoding::types::{Payload, VerifierSet};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::instructions::GatewayInstruction;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::axelar_message::new_signer_set;

use crate::{
    create_signer_set, get_approved_command, get_gateway_events,
    get_gateway_events_from_execute_data, make_messages, make_payload_and_commands,
    setup_initialised_gateway, InitialisedGatewayMetadata,
};

fn payload_and_command(verifier_set: &VerifierSet) -> (Payload, [OwnedCommand; 1]) {
    let payload = Payload::VerifierSet(verifier_set.clone());
    let command = OwnedCommand::RotateSigners(verifier_set.clone());
    (payload, [command])
}

/// successfully process execute when there is 1 rotate signers commands
#[ignore]
#[tokio::test]
async fn successfully_rotates_signers() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let (new_signer_set, new_signers) = create_signer_set(&[500, 200], 700);
    let (payload, command) = payload_and_command(&new_signer_set);

    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let tx = fixture
        .rotate_signers_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pda,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs = get_gateway_events_from_execute_data(&command);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    let approved_command = get_approved_command(&mut fixture, &gateway_approved_command_pda).await;
    assert!(approved_command.is_command_executed());

    // - signers have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = 2u128;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.into()
    );
    assert_eq!(
        root_pda_data
            .auth_weighted
            .signer_set_hash_for_epoch(&new_epoch.into())
            .unwrap(),
        &new_signer_set.hash(),
    );

    // - test that both signer sets can sign new messages
    for signer_set in [new_signers, signers] {
        let messages = make_messages(1);
        fixture
            .fully_approve_messages(&gateway_root_pda, messages, &signer_set)
            .await;
    }
}

/// Ensure that we can use an old signer set to sign messages as long as the
/// operator also signed the `rotate_signers` ix
#[ignore]
#[tokio::test]
async fn succeed_if_signer_set_signed_by_old_signer_set_and_submitted_by_the_operator() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        operator,
        quorum,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    // -- we set a new signer set to be the "latest" signer set
    let (new_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    fixture
        .fully_rotate_signers(&gateway_root_pda, new_signer_set.clone(), &signers)
        .await;

    let (newer_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    let (payload, command) = payload_and_command(&new_signer_set);
    let domain_separator = fixture.domain_separator;
    // we stil use the initial signer set to sign the data (the `signers` variable)
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let ix = gmp_gateway::instructions::rotate_signers(
        gmp_gateway::id(),
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(operator.pubkey()),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&operator, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs = get_gateway_events_from_execute_data(&command);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    let approved_command = get_approved_command(&mut fixture, &rotate_signers_command_pda).await;
    assert!(approved_command.is_command_executed());

    // - signers have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = 3u128;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.into()
    );
    assert_eq!(
        root_pda_data
            .auth_weighted
            .signer_set_hash_for_epoch(&new_epoch.into())
            .unwrap(),
        &newer_signer_set.hash(),
    );
}

/// We use a different account in place of the expected operator to try and
/// rotate signers - but an on-chain check rejects his attempts
#[ignore]
#[tokio::test]
async fn fail_if_provided_operator_is_not_the_real_operator_thats_stored_in_gateway_state() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        quorum,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    // -- we set a new signer set to be the "latest" signer set
    let (new_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    fixture
        .fully_rotate_signers(&gateway_root_pda, new_signer_set.clone(), &signers)
        .await;

    let (newer_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    let (payload, command) = payload_and_command(&newer_signer_set);

    // we stil use the initial signer set to sign the data (the `signers` variable)
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let fake_operator = Keypair::new();
    let ix = gmp_gateway::instructions::rotate_signers(
        gmp_gateway::id(),
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(fake_operator.pubkey()), // `stranger_danger` in place of the expected `operator`
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&fake_operator, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

/// ensure that the operator still needs to use a valid signer set to to
/// force-rotate the signers
#[ignore]
#[tokio::test]
async fn fail_if_operator_is_not_using_pre_registered_signer_set() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        quorum,
        operator,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    // generate a new random operator set to be used (do not register it)
    let (new_signer_set, new_signers) = create_signer_set(&[500, 200], 700);
    let (payload, command) = payload_and_command(&new_signer_set);

    let domain_separator = fixture.domain_separator;
    // using `new_signers` which is the cause of the failure
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &new_signers,
            quorum,
            &domain_separator,
        )
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let ix = gmp_gateway::instructions::rotate_signers(
        gmp_gateway::id(),
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(operator.pubkey()),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&operator, &fixture.payer.insecure_clone()],
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

/// Ensure that the operator also need to explicitly sign the ix
#[ignore]
#[tokio::test]
async fn fail_if_operator_only_passed_but_not_actual_signer() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        operator,
        quorum,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    // -- we set a new signer set to be the "latest" signer set
    let (new_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    fixture
        .fully_rotate_signers(&gateway_root_pda, new_signer_set.clone(), &signers)
        .await;

    let (_, _new_signers) = create_signer_set(&[500, 200], 700);
    let (payload, command) = payload_and_command(&new_signer_set);
    let domain_separator = fixture.domain_separator;

    // we stil use the initial signer set to sign the data (the `signers` variable)
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let data = borsh::to_vec(&GatewayInstruction::RotateSigners).unwrap();
    let accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new(execute_data_pda, false),
        AccountMeta::new(rotate_signers_command_pda, false),
        AccountMeta::new(operator.pubkey(), false), /* the flag being `false` is the cause of
                                                     * failure for the tx */
    ];

    let ix = Instruction {
        program_id: gmp_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(&[ix], &[&fixture.payer.insecure_clone()])
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}
/// disallow rotate signers if any other signer set besides the most recent
/// epoch signed the proof
#[ignore]
#[tokio::test]
async fn fail_if_rotate_signers_signed_by_old_signer_set() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_signer_set, _new_signers) = create_signer_set(&[500, 200], 700);
    fixture
        .fully_rotate_signers(&gateway_root_pda, new_signer_set.clone(), &signers)
        .await;

    // Action
    let (newer_signer_set, _newer_signers) = create_signer_set(&[444, 555], 333);
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            newer_signer_set.clone(),
            &signers,
        )
        .await;

    // Assert
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

/// `rotate_signer_set` is ignored if total weight is smaller than new
/// command weight quorum (tx succeeds)
#[ignore]
#[tokio::test]
async fn ignore_rotate_signers_if_total_weight_is_smaller_than_quorum() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,

        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_signer_set, _signers) = create_signer_set(&[1, 1], 10);

    // Action
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            new_signer_set.clone(),
            &signers,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = 1u128;
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch.into());
    assert_eq!(gateway.auth_weighted.signer_sets().len(), 1);
    assert_ne!(
        gateway
            .auth_weighted
            .signer_set_hash_for_epoch(&constant_epoch.into())
            .unwrap(),
        &new_signer_set.hash(),
    );
}

#[ignore]
#[tokio::test]
async fn fail_if_order_of_commands_is_not_the_same_as_order_of_accounts() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;

    let (payload, commands) = make_payload_and_commands(3);
    let domain_separator = fixture.domain_separator;

    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    // Action
    let mut gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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

/// `rotate_signer_set` is ignored if new signer set len is 0 (tx succeeds)
#[ignore]
#[tokio::test]
async fn fail_on_rotate_signers_if_new_ops_len_is_zero() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;

    let new_signer_set = new_signer_set(&[], 1, 3);
    let (payload, command) = payload_and_command(&new_signer_set);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    // Action
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();
    let tx = fixture
        .rotate_signers_with_metadata(
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
    let constant_epoch = 1u128;
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch.into());
    assert_ne!(
        gateway
            .auth_weighted
            .signer_set_hash_for_epoch(&constant_epoch.into())
            .unwrap(),
        &new_signer_set.hash(),
    );
}

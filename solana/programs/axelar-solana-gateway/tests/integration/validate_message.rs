use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::instructions::validate_message;
use axelar_solana_gateway::state::incoming_message::{
    command_id, IncomingMessageWrapper, MessageStatus,
};
use axelar_solana_gateway::{get_incoming_message_pda, get_validate_message_signing_pda};
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_gateway_test_fixtures::gateway::make_messages;
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use itertools::Itertools;
use solana_program_test::tokio;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn fail_if_message_pda_does_not_exist() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let mut messages = make_messages(1);
    let destination_address = Pubkey::new_unique();
    if let Some(x) = messages.get_mut(0) {
        x.destination_address = destination_address.to_string()
    }
    let message_leaf = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .leaf;
    let fake_command_id = solana_program::keccak::hash(b"fake command id").0; // source of error -- invalid command id
    let (incoming_message_pda, ..) = get_incoming_message_pda(&fake_command_id);

    // action
    let (signing_pda, signing_pda_bump) =
        get_validate_message_signing_pda(destination_address, fake_command_id);
    let ix = validate_message_for_tests(
        &incoming_message_pda,
        &signing_pda,
        message_leaf.message,
        signing_pda_bump,
    )
    .unwrap();
    let err = metadata.send_tx(&[ix]).await.unwrap_err();

    // assert
    assert!(err
        .find_log("incoming message account data is corrupt")
        .is_some());
}

#[tokio::test]
async fn fail_if_message_already_executed() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let mut messages = make_messages(1);
    let destination_address = Pubkey::new_unique();
    if let Some(x) = messages.get_mut(0) {
        x.destination_address = destination_address.to_string()
    }
    let message_leaf = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .leaf;
    let command_id = command_id(
        &message_leaf.message.cc_id.chain,
        &message_leaf.message.cc_id.id,
    );
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id);
    let mut incoming_message = metadata.incoming_message(incoming_message_pda).await;
    incoming_message.message.status = MessageStatus::Executed; // source of error
    set_existing_incoming_message_state(&mut metadata, incoming_message_pda, incoming_message)
        .await;

    // action
    let (signing_pda, signing_pda_bump) =
        get_validate_message_signing_pda(destination_address, command_id);
    let ix = validate_message_for_tests(
        &incoming_message_pda,
        &signing_pda,
        message_leaf.message,
        signing_pda_bump,
    )
    .unwrap();
    let err = metadata.send_tx(&[ix]).await.unwrap_err();

    // assert
    assert!(err.find_log("message not approved").is_some());
}

#[tokio::test]
async fn fail_if_message_has_been_tampered_with() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let mut messages = make_messages(1);
    let destination_address = Pubkey::new_unique();
    if let Some(x) = messages.get_mut(0) {
        x.destination_address = destination_address.to_string()
    }
    let mut message_leaf = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .leaf;
    message_leaf.message.payload_hash = [42; 32]; // source of error
    let command_id = command_id(
        &message_leaf.message.cc_id.chain,
        &message_leaf.message.cc_id.id,
    );
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id);

    // action
    let (signing_pda, signing_pda_bump) =
        get_validate_message_signing_pda(destination_address, command_id);
    let ix = validate_message_for_tests(
        &incoming_message_pda,
        &signing_pda,
        message_leaf.message,
        signing_pda_bump,
    )
    .unwrap();
    let err = metadata.send_tx(&[ix]).await.unwrap_err();

    // assert
    assert!(err.find_log("message has been tampered with").is_some());
}

#[tokio::test]
async fn fail_if_invalid_signing_pda_seeds() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let mut messages = make_messages(1);
    let destination_address = Pubkey::new_unique();
    if let Some(x) = messages.get_mut(0) {
        x.destination_address = destination_address.to_string()
    }
    let message_leaf = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .leaf;
    let command_id = command_id(
        &message_leaf.message.cc_id.chain,
        &message_leaf.message.cc_id.id,
    );
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id);

    // action
    let (signing_pda, signing_pda_bump) =
        get_validate_message_signing_pda(destination_address, [42; 32]); // source of error, invalid command id
    let ix = validate_message_for_tests(
        &incoming_message_pda,
        &signing_pda,
        message_leaf.message,
        signing_pda_bump,
    )
    .unwrap();
    let err = metadata.send_tx(&[ix]).await.unwrap_err();

    // assert
    let err_variant_one = err.find_log("Invalid signing PDA").is_some();
    let err_variant_two = err
        .find_log("Provided seeds do not result in a valid address")
        .is_some();
    // this depends on the bump that gets derived -- sometimes they match, sometimes
    // they don't depending on the random parameters of test run
    let either_error = err_variant_one || err_variant_two;
    assert!(either_error);
}

#[tokio::test]
async fn fail_if_another_valid_message_pda_provided() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let mut messages = make_messages(2);
    let destination_address = Pubkey::new_unique();
    messages
        .iter_mut()
        .for_each(|x| x.destination_address = destination_address.to_string());
    let (message_leaf_one, message_leaf_two) = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages)
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.leaf)
        .next_tuple()
        .unwrap();
    let command_id_leaf_one = command_id(
        &message_leaf_one.message.cc_id.chain,
        &message_leaf_one.message.cc_id.id,
    );
    let command_id_leaf_two = command_id(
        &message_leaf_two.message.cc_id.chain,
        &message_leaf_two.message.cc_id.id,
    );
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id_leaf_two);

    // action
    let (signing_pda, signing_pda_bump) =
        get_validate_message_signing_pda(destination_address, command_id_leaf_one);
    let ix = validate_message_for_tests(
        &incoming_message_pda, // pda of second message
        &signing_pda,
        message_leaf_one.message, // fisrst message
        signing_pda_bump,         // signing pda of first message
    )
    .unwrap();
    let err = metadata.send_tx(&[ix]).await.unwrap_err();

    // assert
    assert!(err.find_log("message has been tampered with").is_some());
}

async fn set_existing_incoming_message_state(
    metadata: &mut axelar_solana_gateway_test_fixtures::SolanaAxelarIntegrationMetadata,
    incoming_message_pda: Pubkey,
    incoming_message: IncomingMessageWrapper,
) {
    let mut raw_account = metadata
        .banks_client
        .get_account(incoming_message_pda)
        .await
        .unwrap()
        .unwrap();
    let incoming_message = bytemuck::bytes_of(&incoming_message);
    raw_account.data = incoming_message.to_vec();
    metadata
        .fixture
        .context
        .set_account(&incoming_message_pda, &raw_account.into());
}

fn validate_message_for_tests(
    incoming_message_pda: &Pubkey,
    signing_pda: &Pubkey,
    message: Message,
    signing_pda_bump: u8,
) -> Result<Instruction, ProgramError> {
    let mut res = validate_message(incoming_message_pda, signing_pda, message, signing_pda_bump)?;
    // needed because we cannot sign with a PDA without creating a real on-chain
    // program
    res.accounts[1].is_signer = false;
    Ok(res)
}
#![cfg(test)]
#![allow(clippy::str_to_string, clippy::indexing_slicing)]
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use axelar_solana_encoding::hasher::MerkleTree;
use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_gateway_v2::executable::{ExecutablePayload, ExecutablePayloadEncodingScheme};
use axelar_solana_gateway_v2::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED;
use axelar_solana_gateway_v2::IncomingMessage;
use axelar_solana_gateway_v2::ID as GATEWAY_PROGRAM_ID;
use axelar_solana_gateway_v2::{CrossChainId, Message, MessageLeaf};
use axelar_solana_gateway_v2_test_fixtures::{
    approve_message_helper, create_verifier_info, initialize_gateway,
    initialize_payload_verification_session_with_root, setup_test_with_real_signers,
    verify_signature_helper,
};
use axelar_solana_memo_v2::Counter;
use axelar_solana_memo_v2::ID as MEMO_PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    system_program::ID as SYSTEM_PROGRAM_ID,
};

#[test]
#[allow(clippy::too_many_lines)]
#[allow(clippy::non_ascii_literal)]
fn test_execute() {
    // Step 0: Example payload
    let memo_string = "🐪🐪🐪🐪";
    let (counter_pda, _counter_pda_bump) = Counter::get_pda();
    let encoding_scheme = ExecutablePayloadEncodingScheme::AbiEncoding;
    let test_payload = ExecutablePayload::new(
        memo_string.as_bytes(),
        &[AccountMeta::new(counter_pda, false)],
        encoding_scheme,
    );
    let test_payload_hash: [u8; 32] = test_payload.hash().unwrap();

    // Step 1: Setup test with real signers
    let (mut setup, verifier_leaves, verifier_merkle_tree, secret_key_1, secret_key_2) =
        setup_test_with_real_signers();

    // Add the memo program to the Mollusk instance
    setup.mollusk.add_program(
        &MEMO_PROGRAM_ID,
        "../../target/deploy/axelar_solana_memo_v2",
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    // Step 2: Initialize gateway
    let init_result = initialize_gateway(&setup);

    // Step 3: Create message merkle tree
    let message = Message {
        cc_id: CrossChainId {
            chain: "ethereum".to_string(),
            id: "memo_msg_1".to_string(),
        },
        source_address: "0x1234567890123456789012345678901234567890".to_string(),
        destination_chain: "solana".to_string(),
        destination_address: MEMO_PROGRAM_ID.to_string(), // This is crucial!
        payload_hash: test_payload_hash,
    };

    let messages = vec![message.clone()];

    let message_leaves: Vec<MessageLeaf> = messages
        .iter()
        .enumerate()
        .map(|(i, msg)| MessageLeaf {
            message: msg.clone(),
            position: u16::try_from(i).unwrap(),
            set_size: u16::try_from(messages.len()).unwrap(),
            domain_separator: setup.domain_separator,
        })
        .collect();

    let message_leaf_hashes: Vec<[u8; 32]> = message_leaves
        .iter()
        .map(axelar_solana_gateway_v2::MessageLeaf::hash)
        .collect();

    let message_merkle_tree = MerkleTree::<SolanaSyscallHasher>::from_leaves(&message_leaf_hashes);
    let payload_merkle_root = message_merkle_tree.root().unwrap();

    // Step 4: Initialize payload verification session
    let (session_result, verification_session_pda) =
        initialize_payload_verification_session_with_root(
            &setup,
            &init_result,
            payload_merkle_root,
        );

    let gateway_root_account = init_result.get_account(&setup.gateway_root_pda).unwrap();

    let verifier_set_tracker_account = init_result
        .get_account(&setup.verifier_set_tracker_pda)
        .unwrap();

    let verification_session_account = session_result
        .get_account(&verification_session_pda)
        .unwrap();

    // Step 5: Sign the payload with both signers, verify both signatures on the gateway
    let verifier_info_1 = create_verifier_info(
        &secret_key_1,
        payload_merkle_root,
        &verifier_leaves[0],
        0, // Position 0
        &verifier_merkle_tree,
    );

    let verify_result_1 = verify_signature_helper(
        &setup,
        payload_merkle_root,
        verifier_info_1,
        verification_session_pda,
        gateway_root_account.clone(),
        verification_session_account.clone(),
        setup.verifier_set_tracker_pda,
        verifier_set_tracker_account.clone(),
    );

    let updated_verification_account_after_first = verify_result_1
        .get_account(&verification_session_pda)
        .unwrap();

    let verifier_info_2 = create_verifier_info(
        &secret_key_2,
        payload_merkle_root,
        &verifier_leaves[1],
        1, // Position 1
        &verifier_merkle_tree,
    );

    let verify_result_2 = verify_signature_helper(
        &setup,
        payload_merkle_root,
        verifier_info_2,
        verification_session_pda,
        gateway_root_account.clone(),
        updated_verification_account_after_first.clone(),
        setup.verifier_set_tracker_pda,
        verifier_set_tracker_account.clone(),
    );

    // Step 6: Approve the message
    let (approve_result, incoming_message_pda) = approve_message_helper(
        &setup,
        message_merkle_tree,
        message_leaves,
        &messages,
        payload_merkle_root,
        verification_session_pda,
        verify_result_2,
        0, // position
    );

    assert!(
        !approve_result.program_result.is_err(),
        "Message approval should succeed"
    );

    let incoming_message_account = approve_result.get_account(&incoming_message_pda).unwrap();

    let incoming_message =
        IncomingMessage::try_deserialize(&mut incoming_message_account.data.as_slice()).unwrap();

    // Step 7.1: Init Counter PDA
    let init_ix = axelar_solana_memo_v2::instruction::Init {};
    let init_accounts = axelar_solana_memo_v2::accounts::Init {
        counter: counter_pda,
        payer: setup.payer,
        system_program: SYSTEM_PROGRAM_ID,
    };
    let init_instruction = Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: init_accounts.to_account_metas(None),
        data: init_ix.data(),
    };
    let init_accounts = vec![
        (
            counter_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.payer,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            SYSTEM_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ];

    let init_result = setup
        .mollusk
        .process_instruction(&init_instruction, &init_accounts);

    assert!(init_result.program_result.is_ok());

    let counter_pda_account = init_result.get_account(&counter_pda).unwrap();

    // Step 7.2: Execute the message
    let message = &messages[0];
    let command_id = message.command_id();

    let signing_pda = Pubkey::create_program_address(
        &[
            VALIDATE_MESSAGE_SIGNING_SEED,
            command_id.as_ref(),
            &[incoming_message.signing_pda_bump],
        ],
        &MEMO_PROGRAM_ID,
    )
    .unwrap();

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let execute_instruction_data = axelar_solana_memo_v2::instruction::Execute {
        message: message.clone(),
        payload: test_payload.payload_without_accounts().to_vec(),
        encoding_scheme,
    }
    .data();

    let execute_accounts = vec![
        (incoming_message_pda, incoming_message_account.clone()),
        (
            signing_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: GATEWAY_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            GATEWAY_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            event_authority_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (setup.gateway_root_pda, gateway_root_account.clone()),
        (
            MEMO_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (counter_pda, counter_pda_account.clone()),
    ];

    let execute_ix_accounts = axelar_solana_memo_v2::accounts::Execute {
        executable: axelar_solana_memo_v2::accounts::AxelarExecuteAccounts {
            incoming_message_pda,
            signing_pda,
            gateway_root_pda: setup.gateway_root_pda,
            axelar_gateway_program: GATEWAY_PROGRAM_ID,
            event_authority: event_authority_pda,
        },
        counter: counter_pda,
    }
    .to_account_metas(None);

    let execute_instruction = Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: execute_ix_accounts,
        data: execute_instruction_data,
    };

    let execute_result = setup
        .mollusk
        .process_instruction(&execute_instruction, &execute_accounts);

    assert!(
        !execute_result.program_result.is_err(),
        "Execute instruction should succeed: {:?}",
        execute_result.program_result
    );

    let counter_pda_account = execute_result.get_account(&counter_pda).unwrap();

    let counter_data = Counter::try_deserialize(&mut counter_pda_account.data.as_slice()).unwrap();
    assert_eq!(
        counter_data.counter, 1,
        "Counter should be incremented to 1"
    );

    // TODO test event cpi
}

use anchor_lang::{solana_program, AccountDeserialize};
use axelar_solana_gateway_v2_test_fixtures::{
    approve_messages_on_gateway, create_test_message, initialize_gateway,
    setup_test_with_real_signers,
};
use axelar_solana_governance_v2::state::GovernanceConfigInit;
use axelar_solana_governance_v2::ExecutableProposal;
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use axelar_solana_governance_v2_test_fixtures::{
    create_gateway_event_authority_pda, create_governance_config_pda,
    create_governance_event_authority_pda, create_governance_program_data_pda,
    create_operator_proposal_pda, create_proposal_pda, create_signing_pda_from_message,
    extract_proposal_hash_unchecked, initialize_governance, process_gmp_helper, GmpContext,
    TestSetup,
};
use hex::FromHex;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_full_governance_workflow_schedule_and_approve_operator() {
    // Step 1: Setup gateway with real signers
    let (mut setup, verifier_leaves, verifier_merkle_tree, secret_key_1, secret_key_2) =
        setup_test_with_real_signers();

    // Step 2: Initialize gateway
    let init_result = initialize_gateway(&setup);
    let gateway_root = init_result
        .get_account(&setup.gateway_root_pda)
        .unwrap()
        .clone();
    assert!(!init_result.program_result.is_err());

    // Step 3: Create ALL 4 governance message payloads
    // These payloads represent the same proposal but different commands

    // Schedule timelock proposal (command = 0)
    let schedule_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d40fa100000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let schedule_payload_hash = solana_program::keccak::hashv(&[&schedule_payload]).to_bytes();

    // Cancel timelock proposal (command = 1)
    let cancel_timelock_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d69bdc00000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let cancel_timelock_payload_hash =
        solana_program::keccak::hashv(&[&cancel_timelock_payload]).to_bytes();

    // Approve operator proposal (command = 2)
    let approve_operator_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d40fa100000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let approve_operator_payload_hash =
        solana_program::keccak::hashv(&[&approve_operator_payload]).to_bytes();

    // Cancel operator approval (command = 3)
    let cancel_operator_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d40fa100000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let cancel_operator_payload_hash =
        solana_program::keccak::hashv(&[&cancel_operator_payload]).to_bytes();

    // Step 4: Create messages for all 4 commands
    let messages = vec![
        create_test_message(
            "ethereum",
            "schedule_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            schedule_payload_hash,
        ),
        create_test_message(
            "ethereum",
            "cancel_timelock_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            cancel_timelock_payload_hash,
        ),
        create_test_message(
            "ethereum",
            "approve_operator_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            approve_operator_payload_hash,
        ),
        create_test_message(
            "ethereum",
            "cancel_operator_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            cancel_operator_payload_hash,
        ),
    ];

    let incoming_messages = approve_messages_on_gateway(
        &setup,
        messages.clone(),
        init_result,
        &secret_key_1,
        &secret_key_2,
        verifier_leaves,
        verifier_merkle_tree,
    );

    // Step 10: Setup Governance
    setup.mollusk.add_program(
        &GOVERNANCE_PROGRAM_ID,
        "../../target/deploy/axelar_solana_governance_v2",
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    let payer = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let operator = Pubkey::new_unique();

    let (governance_config, governance_config_bump) = create_governance_config_pda();

    let program_data_pda = create_governance_program_data_pda();

    let (event_authority_pda_governance, event_authority_bump) =
        create_governance_event_authority_pda();

    let governance_setup = TestSetup {
        mollusk: setup.mollusk,
        payer,
        upgrade_authority,
        operator,
        governance_config,
        governance_config_bump,
        program_data_pda,
        event_authority_pda: event_authority_pda_governance,
        event_authority_bump,
    };

    let chain_hash = solana_program::keccak::hashv(&[b"ethereum"]).to_bytes();
    let address_hash =
        solana_program::keccak::hashv(&["0xSourceAddress".to_string().as_bytes()]).to_bytes();
    let minimum_proposal_eta_delay = 3600;

    let governance_config = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        operator.to_bytes(),
    );

    let init_governance_result = initialize_governance(&governance_setup, governance_config);
    assert!(!init_governance_result.program_result.is_err());

    let governance_config_account = init_governance_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    let schedule_incoming_message = incoming_messages[0].clone().0;
    let schedule_incoming_message_pda = incoming_messages[0].1;
    let schedule_incoming_message_account_data = incoming_messages[0].clone().2;

    // Process SCHEDULE timelock proposal first
    let schedule_message = messages[0].clone();
    let schedule_signing_pda =
        create_signing_pda_from_message(&schedule_message, &schedule_incoming_message);
    let event_authority_pda_gateway = create_gateway_event_authority_pda();
    let proposal_hash = extract_proposal_hash_unchecked(&schedule_payload);
    let proposal_pda = create_proposal_pda(&proposal_hash);

    let gmp_context = GmpContext::new()
        .with_incoming_message(
            schedule_incoming_message_pda,
            schedule_incoming_message_account_data.clone(),
        )
        .with_governance_config(
            governance_setup.governance_config,
            governance_config_account.data.clone(),
        )
        .with_gateway_root_pda(setup.gateway_root_pda, gateway_root.data.clone())
        .with_signing_pda(schedule_signing_pda)
        .with_event_authority_pda(event_authority_pda_gateway)
        .with_event_authority_pda_governance(event_authority_pda_governance)
        .with_proposal(proposal_pda, vec![], SYSTEM_PROGRAM_ID);

    // Send schedule timelock proposal
    let schedule_result = process_gmp_helper(
        &governance_setup,
        schedule_message,
        schedule_payload,
        gmp_context,
    );
    assert!(!schedule_result.program_result.is_err());

    let proposal_pda_account_after_schedule = schedule_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    let _ = ExecutableProposal::try_deserialize(
        &mut proposal_pda_account_after_schedule.data.as_slice(),
    )
    .unwrap();

    // Now process APPROVE operator proposal
    let approve_operator_message = messages[2].clone();
    let approve_operator_incoming_message = incoming_messages[2].clone().0;
    let approve_operator_incoming_message_pda = incoming_messages[2].1;
    let approve_operator_incoming_message_account_data = incoming_messages[2].clone().2;

    let approve_operator_signing_pda = create_signing_pda_from_message(
        &approve_operator_message,
        &approve_operator_incoming_message,
    );

    let operator_proposal_pda = create_operator_proposal_pda(&proposal_hash);

    let gmp_context = GmpContext::new()
        .with_incoming_message(
            approve_operator_incoming_message_pda,
            approve_operator_incoming_message_account_data,
        )
        .with_governance_config(
            governance_setup.governance_config,
            governance_config_account.data.clone(),
        )
        .with_gateway_root_pda(setup.gateway_root_pda, gateway_root.data.clone())
        .with_signing_pda(approve_operator_signing_pda)
        .with_event_authority_pda(event_authority_pda_gateway)
        .with_event_authority_pda_governance(event_authority_pda_governance)
        .with_proposal(
            proposal_pda,
            proposal_pda_account_after_schedule.data,
            GOVERNANCE_PROGRAM_ID,
        )
        .with_operator_proposal(operator_proposal_pda, vec![], SYSTEM_PROGRAM_ID);

    // Send approve operator proposal
    let approve_operator_result = process_gmp_helper(
        &governance_setup,
        approve_operator_message,
        approve_operator_payload,
        gmp_context,
    );
    assert!(!approve_operator_result.program_result.is_err());

    let operator_proposal_pda_account = approve_operator_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == operator_proposal_pda)
        .unwrap()
        .1
        .clone();

    assert!(
        !operator_proposal_pda_account.data.is_empty(),
        "Operator proposal PDA should be created"
    );

    let operator_proposal = axelar_solana_governance_v2::OperatorProposal::try_deserialize(
        &mut operator_proposal_pda_account.data.as_slice(),
    );
    assert!(
        operator_proposal.is_ok(),
        "Should be able to deserialize OperatorProposal"
    );

    let cancel_operator_incoming_message = incoming_messages[3].clone().0;
    let cancel_operator_incoming_message_pda = incoming_messages[3].1;
    let cancel_operator_incoming_message_account_data = incoming_messages[3].clone().2;

    // Now approve the CANCEL operator proposal message and process it
    let cancel_operator_message = messages[3].clone();
    let cancel_operator_signing_pda = create_signing_pda_from_message(
        &cancel_operator_message,
        &cancel_operator_incoming_message,
    );

    // Get the current proposal account data (after approve operator)
    let proposal_pda_account_after_approve = approve_operator_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    let gmp_context = GmpContext::new()
        .with_incoming_message(
            cancel_operator_incoming_message_pda,
            cancel_operator_incoming_message_account_data,
        )
        .with_governance_config(
            governance_setup.governance_config,
            governance_config_account.data,
        )
        .with_gateway_root_pda(setup.gateway_root_pda, gateway_root.data.clone())
        .with_signing_pda(cancel_operator_signing_pda)
        .with_event_authority_pda(event_authority_pda_gateway)
        .with_event_authority_pda_governance(event_authority_pda_governance)
        .with_proposal(
            proposal_pda,
            proposal_pda_account_after_approve.data,
            GOVERNANCE_PROGRAM_ID,
        )
        .with_operator_proposal(
            operator_proposal_pda,
            operator_proposal_pda_account.data,
            GOVERNANCE_PROGRAM_ID,
        );

    //  Send CANCEL operator proposal
    let cancel_operator_result = process_gmp_helper(
        &governance_setup,
        cancel_operator_message,
        cancel_operator_payload,
        gmp_context,
    );
    assert!(!cancel_operator_result.program_result.is_err());

    // Verify that the operator proposal PDA was closed
    let operator_proposal_account_after_cancel = cancel_operator_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == operator_proposal_pda);

    assert!(
        operator_proposal_account_after_cancel.is_none()
            || operator_proposal_account_after_cancel
                .unwrap()
                .1
                .data
                .is_empty(),
        "Operator proposal PDA should be closed after cancel"
    );

    // Verify the governance config account received the lamports from the closed account
    let governance_config_after_cancel = cancel_operator_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    assert!(
        governance_config_after_cancel.lamports >= governance_config_account.lamports,
        "Governance config should have received lamports from closed operator proposal account"
    );
}

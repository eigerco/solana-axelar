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
    create_governance_event_authority_pda, create_governance_program_data_pda, create_proposal_pda,
    create_signing_pda_from_message, extract_proposal_hash_unchecked, initialize_governance,
    process_gmp_helper, GmpContext, TestSetup,
};
use hex::FromHex;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_schedule_timelock_proposal() {
    // Step 1: Setup gateway with real signers
    let (mut setup, verifier_leaves, verifier_merkle_tree, secret_key_1, secret_key_2) =
        setup_test_with_real_signers();

    // Step 2: Initialize gateway
    let init_result = initialize_gateway(&setup);
    assert!(!init_result.program_result.is_err());
    let gateway_root = init_result
        .get_account(&setup.gateway_root_pda)
        .unwrap()
        .clone();

    let schedule_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d40fa100000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let schedule_payload_hash = solana_program::keccak::hashv(&[&schedule_payload]).to_bytes();

    let cancel_payload: Vec<u8> = Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000068d69bdc00000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a36000000000000000000000000000000000000000000000000000000000000004e010000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000").unwrap();
    let cancel_payload_hash = solana_program::keccak::hashv(&[&cancel_payload]).to_bytes();

    let messages = vec![
        create_test_message(
            "ethereum",
            "msg_id_1",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            schedule_payload_hash,
        ),
        create_test_message(
            "ethereum",
            "msg_id_2",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            cancel_payload_hash,
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

    // Now we have an approved message
    // Setup Governance
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

    let chain_hash = solana_program::keccak::hashv(&[b"ethereum"]).to_bytes();
    let address_hash =
        solana_program::keccak::hashv(&["0xSourceAddress".to_string().as_bytes()]).to_bytes();
    let minimum_proposal_eta_delay = 3600;

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

    let governance_config = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        operator.to_bytes(),
    );

    let result = initialize_governance(&governance_setup, governance_config);
    assert!(!result.program_result.is_err());

    let governance_config_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    let message = messages[0].clone();
    let schedule_incoming_message_account = incoming_messages[0].clone().0;
    let schedule_incoming_message_pda = incoming_messages[0].clone().1;
    let schedule_incoming_message_account_data = incoming_messages[0].clone().2;

    let signing_pda = create_signing_pda_from_message(&message, &schedule_incoming_message_account);
    let event_authority_pda_gateway = create_gateway_event_authority_pda();
    let (event_authority_pda_governance, _) = create_governance_event_authority_pda();
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
        .with_signing_pda(signing_pda)
        .with_event_authority_pda(event_authority_pda_gateway)
        .with_event_authority_pda_governance(event_authority_pda_governance)
        .with_proposal(proposal_pda, vec![], SYSTEM_PROGRAM_ID);

    // Send schedule timelock proposal
    let result = process_gmp_helper(
        &governance_setup,
        messages[0].clone(),
        schedule_payload,
        gmp_context,
    );
    assert!(!result.program_result.is_err());

    let proposal_pda_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    let _ = ExecutableProposal::try_deserialize(&mut proposal_pda_account.data.as_slice()).unwrap();

    let cancel_message = messages[1].clone();
    let cancel_incoming_message = incoming_messages[1].clone().0;
    let cancel_incoming_message_pda = incoming_messages[1].1;
    let cancel_incoming_message_account_data = incoming_messages[1].clone().2;

    let cancel_signing_pda =
        create_signing_pda_from_message(&cancel_message, &cancel_incoming_message);

    let gmp_context = GmpContext::new()
        .with_incoming_message(
            cancel_incoming_message_pda,
            cancel_incoming_message_account_data,
        )
        .with_governance_config(
            governance_setup.governance_config,
            governance_config_account.data,
        )
        .with_gateway_root_pda(setup.gateway_root_pda, gateway_root.data.clone())
        .with_signing_pda(cancel_signing_pda)
        .with_event_authority_pda(event_authority_pda_gateway)
        .with_event_authority_pda_governance(event_authority_pda_governance)
        .with_proposal(
            proposal_pda,
            proposal_pda_account.data,
            GOVERNANCE_PROGRAM_ID,
        );

    // Send cancel timelock proposal
    let result = process_gmp_helper(
        &governance_setup,
        messages[1].clone(),
        cancel_payload,
        gmp_context,
    );
    assert!(!result.program_result.is_err());

    let proposal_pda_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    assert_eq!(proposal_pda_account.data.len(), 0);
}

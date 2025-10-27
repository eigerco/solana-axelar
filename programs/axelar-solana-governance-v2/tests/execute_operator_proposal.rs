use alloy_sol_types::SolValue;
use anchor_lang::prelude::AccountMeta;
use anchor_lang::AnchorSerialize;
use anchor_lang::{solana_program, AccountDeserialize, ToAccountMetas};
use axelar_solana_gateway_v2::IncomingMessage;
use axelar_solana_gateway_v2_test_fixtures::{
    approve_messages_on_gateway, create_test_message, initialize_gateway,
    setup_test_with_real_signers,
};
use axelar_solana_governance_v2::state::GovernanceConfigInit;
use axelar_solana_governance_v2::SolanaAccountMetadata;
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use axelar_solana_governance_v2_test_fixtures::{
    create_execute_operator_proposal_instruction_data, create_gateway_event_authority_pda,
    create_governance_config_pda, create_governance_event_authority_pda,
    create_governance_program_data_pda, create_operator_proposal_pda, create_proposal_pda,
    create_signing_pda_from_message, extract_proposal_hash_unchecked, get_memo_instruction_data,
    initialize_governance, process_gmp_helper, GmpContext, TestSetup,
};
use axelar_solana_memo_v2::ID as MEMO_PROGRAM_ID;
use governance_gmp::alloy_primitives::U256;
use solana_sdk::account::Account;
use solana_sdk::instruction::Instruction;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_execute_operator_proposal() {
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

    // Step 3: Create governance message payloads for schedule and approve operator
    let memo = String::from("This is a operator proposal memo");
    let native_value_u64 = 2;
    let eta = 1800000000;

    let value_receiver_pubkey = Pubkey::new_unique();
    let value_receiver = SolanaAccountMetadata {
        pubkey: value_receiver_pubkey.to_bytes(),
        is_signer: false,
        is_writable: true,
    };
    let target_bytes: [u8; 32] = MEMO_PROGRAM_ID.to_bytes();
    let call_data = get_memo_instruction_data(memo, value_receiver);
    let native_value = U256::from(native_value_u64);
    let eta = U256::from(eta);

    // Schedule timelock proposal payload
    let schedule_gmp_payload = governance_gmp::GovernanceCommandPayload {
        command: governance_gmp::GovernanceCommand::ScheduleTimeLockProposal,
        target: target_bytes.to_vec().into(),
        call_data: call_data.try_to_vec().unwrap().into(),
        native_value,
        eta,
    };
    let schedule_payload = schedule_gmp_payload.abi_encode();
    let schedule_payload_hash = solana_program::keccak::hashv(&[&schedule_payload]).to_bytes();

    // Approve operator proposal payload (same target/call_data/native_value)
    let approve_operator_gmp_payload = governance_gmp::GovernanceCommandPayload {
        command: governance_gmp::GovernanceCommand::ApproveOperatorProposal,
        target: target_bytes.to_vec().into(),
        call_data: call_data.try_to_vec().unwrap().into(),
        native_value,
        eta,
    };
    let approve_operator_payload = approve_operator_gmp_payload.abi_encode();
    let approve_operator_payload_hash =
        solana_program::keccak::hashv(&[&approve_operator_payload]).to_bytes();

    let messages = vec![
        create_test_message(
            "ethereum",
            "schedule_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            schedule_payload_hash,
        ),
        create_test_message(
            "ethereum",
            "approve_operator_msg",
            &GOVERNANCE_PROGRAM_ID.to_string(),
            approve_operator_payload_hash,
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

    // Step 7: Setup Governance
    setup.mollusk.add_program(
        &GOVERNANCE_PROGRAM_ID,
        "../../target/deploy/axelar_solana_governance_v2",
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    setup.mollusk.add_program(
        &MEMO_PROGRAM_ID,
        "../../target/deploy/axelar_solana_memo_v2",
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

    let governance_config_data = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        operator.to_bytes(),
    );

    let init_governance_result = initialize_governance(&governance_setup, governance_config_data);
    assert!(!init_governance_result.program_result.is_err());

    let governance_config_account = init_governance_result
        .get_account(&governance_setup.governance_config)
        .unwrap()
        .clone();

    // Step 8: Process SCHEDULE timelock proposal
    let schedule_message = messages[0].clone();
    let schedule_incoming_message_pda = incoming_messages[0].1;
    let schedule_incoming_message_account_data = incoming_messages[0].clone().2;

    let schedule_incoming_message =
        IncomingMessage::try_deserialize(&mut schedule_incoming_message_account_data.as_slice())
            .unwrap();

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

    let proposal_pda_account_after_schedule =
        schedule_result.get_account(&proposal_pda).unwrap().clone();

    // Step 9: Process APPROVE operator proposal
    let approve_operator_message = messages[1].clone();
    let approve_operator_incoming_message_pda = incoming_messages[1].1;
    let approve_operator_incoming_message_account_data = incoming_messages[1].clone().2;

    let approve_operator_incoming_message = IncomingMessage::try_deserialize(
        &mut approve_operator_incoming_message_account_data.as_slice(),
    )
    .unwrap();

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

    // Step 10: Execute the operator proposal
    let instruction_data = create_execute_operator_proposal_instruction_data(
        MEMO_PROGRAM_ID.to_bytes(),
        call_data.clone(),
        native_value.to_le_bytes(),
    );

    // Get updated accounts
    let governance_config_account_updated = approve_operator_result
        .get_account(&governance_setup.governance_config)
        .unwrap()
        .clone();

    let proposal_pda_account_updated = approve_operator_result
        .get_account(&proposal_pda)
        .unwrap()
        .clone();

    let operator_proposal_account_updated = approve_operator_result
        .get_account(&operator_proposal_pda)
        .unwrap()
        .clone();

    // Set up accounts for execute operator proposal instruction
    let accounts = vec![
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
        (
            governance_setup.governance_config,
            governance_config_account_updated,
        ),
        (proposal_pda, proposal_pda_account_updated),
        (
            operator,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (operator_proposal_pda, operator_proposal_account_updated),
        (
            event_authority_pda_governance,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            GOVERNANCE_PROGRAM_ID,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        // Remaining accounts
        (
            MEMO_PROGRAM_ID,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            value_receiver_pubkey,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
    ];

    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: axelar_solana_governance_v2::accounts::ExecuteOperatorProposal {
            system_program: SYSTEM_PROGRAM_ID,
            governance_config: governance_setup.governance_config,
            proposal_pda,
            operator,
            operator_pda_marker_account: operator_proposal_pda,
            event_authority: event_authority_pda_governance,
            program: GOVERNANCE_PROGRAM_ID,
        }
        .to_account_metas(None)
        .into_iter()
        .chain(vec![
            // Remaining accounts
            AccountMeta::new_readonly(MEMO_PROGRAM_ID, false),
            AccountMeta::new(value_receiver_pubkey, false),
            AccountMeta::new(governance_setup.governance_config, false),
        ])
        .collect(),
        data: instruction_data,
    };

    let execute_result = governance_setup
        .mollusk
        .process_instruction(&instruction, &accounts);

    assert!(
        !execute_result.program_result.is_err(),
        "Execute operator proposal should succeed: {:?}",
        execute_result.program_result
    );

    // Verify both PDAs were closed
    let proposal_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap();

    assert_eq!(
        proposal_account_after_execution.1.data.len(),
        0,
        "Proposal PDA should be closed after execution"
    );
    assert_eq!(
        proposal_account_after_execution.1.lamports, 0,
        "Proposal PDA should have zero lamports after execution"
    );

    let operator_proposal_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == operator_proposal_pda)
        .unwrap();

    assert_eq!(
        operator_proposal_account_after_execution.1.data.len(),
        0,
        "Operator proposal PDA should be closed after execution"
    );
    assert_eq!(
        operator_proposal_account_after_execution.1.lamports, 0,
        "Operator proposal PDA should have zero lamports after execution"
    );

    // Verify lamport transfer to value receiver
    let value_receiver_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == value_receiver_pubkey)
        .unwrap();

    assert_eq!(
        value_receiver_account_after_execution.1.lamports, native_value_u64,
        "Value receiver should have received native value lamports"
    );

    // Verify governance config received the closed PDA lamports
    let governance_config_account_after = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_config)
        .unwrap();

    assert!(
        governance_config_account_after.1.lamports > governance_config_account.lamports,
        "Governance config should have received lamports from closed PDAs"
    );
}

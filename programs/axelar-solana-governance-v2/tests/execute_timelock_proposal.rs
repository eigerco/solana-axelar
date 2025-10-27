use alloy_sol_types::SolValue;
use anchor_lang::prelude::{AccountMeta, ToAccountMetas};
use anchor_lang::solana_program;
use anchor_lang::AnchorSerialize;
use axelar_solana_gateway_v2_test_fixtures::{
    approve_messages_on_gateway, create_test_message, initialize_gateway,
    setup_test_with_real_signers,
};
use axelar_solana_governance_v2::seed_prefixes::GOVERNANCE_CONFIG;
use axelar_solana_governance_v2::state::GovernanceConfigInit;
use axelar_solana_governance_v2::SolanaAccountMetadata;
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use axelar_solana_governance_v2_test_fixtures::{
    create_execute_proposal_instruction_data, create_gateway_event_authority_pda,
    create_governance_event_authority_pda, create_governance_program_data_pda, create_proposal_pda,
    create_signing_pda_from_message, extract_proposal_hash_unchecked, get_memo_instruction_data,
    initialize_governance, process_gmp_helper, GmpContext, TestSetup,
};
use axelar_solana_memo_v2::ID as MEMO_PROGRAM_ID;
use governance_gmp::alloy_primitives::U256;
use hex::FromHex;
use solana_sdk::account::Account;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_execute_scheduled_proposal() {
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

    // Step 3: Create the memo proposal data
    let memo = String::from("This is a sample memo");
    let native_value_u64 = 1;
    let eta = 1800000000;

    let value_receiver_pubkey = Pubkey::new_unique();

    let value_receiver = SolanaAccountMetadata {
        pubkey: value_receiver_pubkey.to_bytes(),
        is_signer: false,
        is_writable: true,
    };

    let call_data = get_memo_instruction_data(memo, value_receiver);
    let target_bytes: [u8; 32] = MEMO_PROGRAM_ID.to_bytes();
    let native_value = U256::from(native_value_u64);
    let eta = U256::from(eta);

    let gmp_payload = governance_gmp::GovernanceCommandPayload {
        command: governance_gmp::GovernanceCommand::ScheduleTimeLockProposal,
        target: target_bytes.to_vec().into(),
        call_data: call_data.try_to_vec().unwrap().into(),
        native_value,
        eta,
    };

    // Encode the GMP payload
    let schedule_payload = gmp_payload.abi_encode();
    let schedule_payload_hash = solana_program::keccak::hashv(&[&schedule_payload]).to_bytes();

    let other_payload: Vec<u8> = Vec::from_hex("DEADBEEF").unwrap();
    let other_payload_hash = solana_program::keccak::hashv(&[&other_payload]).to_bytes();

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
            other_payload_hash,
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
    // Add remaining programs to mollusk
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

    let (governance_config_pda, governance_config_bump) =
        Pubkey::find_program_address(&[GOVERNANCE_CONFIG], &GOVERNANCE_PROGRAM_ID);

    let program_data_pda = create_governance_program_data_pda();

    let (event_authority_pda_governance, event_authority_bump) =
        create_governance_event_authority_pda();

    let chain_hash = solana_program::keccak::hashv(&[b"ethereum"]).to_bytes();
    let address_hash =
        solana_program::keccak::hashv(&["0xSourceAddress".to_string().as_bytes()]).to_bytes();
    let minimum_proposal_eta_delay = 3600;

    let mut governance_setup = TestSetup {
        mollusk: setup.mollusk,
        payer,
        upgrade_authority,
        operator,
        governance_config: governance_config_pda,
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
    let schedule_incoming_message = incoming_messages[0].clone().0;
    let schedule_incoming_message_pda = incoming_messages[0].1;
    let schedule_incoming_message_account_data = incoming_messages[0].clone().2;

    let signing_pda = create_signing_pda_from_message(&message, &schedule_incoming_message);
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

    // Create execute proposal instruction discriminator
    let instruction_data = create_execute_proposal_instruction_data(
        MEMO_PROGRAM_ID.to_bytes(),
        call_data.clone(),
        native_value.to_le_bytes(),
    );

    // Get the updated governance config account
    let governance_config_account_updated = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    // Get the proposal PDA account after scheduling
    let proposal_pda_account_updated = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    // Set up accounts for execute proposal instruction
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
        accounts: axelar_solana_governance_v2::accounts::ExecuteProposal {
            system_program: SYSTEM_PROGRAM_ID,
            governance_config: governance_setup.governance_config,
            proposal_pda,
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

    // Convert eta from U256 to i64 for timestamp comparison
    let eta_timestamp: i64 = eta.try_into().unwrap_or(1800000000i64);

    // Make Mollusk think we're in the future by modifying its clock sysvar
    let current_timestamp = eta_timestamp + 3600; // 1 hour past ETA to be safe
    governance_setup.mollusk.sysvars.clock = Clock {
        slot: 1000,
        epoch_start_timestamp: eta_timestamp,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: current_timestamp,
    };

    let governance_config_account_before = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_config_pda)
        .unwrap();

    let execute_result = governance_setup
        .mollusk
        .process_instruction(&instruction, &accounts);

    assert!(
        !execute_result.program_result.is_err(),
        "Execute proposal should succeed: {:?}",
        execute_result.program_result
    );

    // Verify the proposal PDA was closed
    let proposal_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap();

    assert_eq!(proposal_account_after_execution.1.data.len(), 0);

    assert_eq!(
        proposal_account_after_execution.1.lamports, 0,
        "Proposal PDA should be closed after execution"
    );

    let value_receiver_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == value_receiver_pubkey)
        .unwrap();

    let governance_config_account_after = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_config_pda)
        .unwrap();

    // Governance config should get closed PDA lamports
    assert!(
        governance_config_account_after.1.lamports > governance_config_account_before.1.lamports
    );
    assert!(value_receiver_account_after_execution.1.lamports == native_value_u64);
}

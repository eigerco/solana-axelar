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
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use axelar_solana_governance_v2_test_fixtures::{
    create_execute_proposal_instruction_data, create_gateway_event_authority_pda,
    create_governance_event_authority_pda, create_governance_program_data_pda, create_proposal_pda,
    create_signing_pda_from_message, extract_proposal_hash_unchecked,
    get_withdraw_tokens_instruction_data, initialize_governance, mock_setup_test,
    process_gmp_helper, GmpContext, TestSetup,
};
use governance_gmp::alloy_primitives::U256;
use solana_sdk::account::Account;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_execute_withdraw_tokens_through_proposal() {
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

    // Step 3: Create the withdraw tokens proposal data
    let withdraw_amount = 5_000_000u64; // 0.005 SOL
    let native_value_u64 = 0;
    let eta = 1800000000;

    let receiver_pubkey = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let operator = Pubkey::new_unique();

    let (governance_config_pda, governance_config_bump) =
        Pubkey::find_program_address(&[GOVERNANCE_CONFIG], &GOVERNANCE_PROGRAM_ID);

    let governance_config_bytes: [u8; 32] = governance_config_pda.to_bytes();
    let native_value = U256::from(native_value_u64);
    let eta = U256::from(eta);

    let call_data = get_withdraw_tokens_instruction_data(
        withdraw_amount,
        receiver_pubkey,
        governance_config_bytes,
    );

    // We want our proposal to execute on Governance itself
    let target_program_bytes = GOVERNANCE_PROGRAM_ID.to_bytes();

    let gmp_payload = governance_gmp::GovernanceCommandPayload {
        command: governance_gmp::GovernanceCommand::ScheduleTimeLockProposal,
        target: target_program_bytes.to_vec().into(),
        call_data: call_data.try_to_vec().unwrap().into(),
        native_value,
        eta,
    };

    let schedule_payload = gmp_payload.abi_encode();
    let schedule_payload_hash = solana_program::keccak::hashv(&[&schedule_payload]).to_bytes();

    let messages = vec![create_test_message(
        "ethereum",
        "withdraw_msg",
        &GOVERNANCE_PROGRAM_ID.to_string(),
        schedule_payload_hash,
    )];

    let incoming_messages = approve_messages_on_gateway(
        &setup,
        messages.clone(),
        init_result,
        &secret_key_1,
        &secret_key_2,
        verifier_leaves,
        verifier_merkle_tree,
    );

    let incoming_message = incoming_messages[0].clone();

    // Step 9: Setup Governance
    setup.mollusk.add_program(
        &GOVERNANCE_PROGRAM_ID,
        "../../target/deploy/axelar_solana_governance_v2",
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

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

    let governance_config_data = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        operator.to_bytes(),
    );

    let result = initialize_governance(&governance_setup, governance_config_data);
    assert!(!result.program_result.is_err());

    // Give governance config some lamports to withdraw
    let mut governance_config_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    let treasury_amount = 10_000_000u64; // Give it 0.01 SOL
    governance_config_account.lamports += treasury_amount;

    // Step 10: Schedule the withdraw proposal
    let message = messages[0].clone();

    let signing_pda = create_signing_pda_from_message(&message, &incoming_message.0.clone());
    let event_authority_pda_gateway = create_gateway_event_authority_pda();
    let proposal_hash = extract_proposal_hash_unchecked(&schedule_payload);
    let proposal_pda = create_proposal_pda(&proposal_hash);

    let gmp_context = GmpContext::new()
        .with_incoming_message(incoming_message.1, incoming_message.2.clone())
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
    let schedule_result = process_gmp_helper(
        &governance_setup,
        messages[0].clone(),
        schedule_payload,
        gmp_context,
    );
    assert!(!schedule_result.program_result.is_err());

    // Step 11: Execute the proposal after ETA
    let instruction_data = create_execute_proposal_instruction_data(
        GOVERNANCE_PROGRAM_ID.to_bytes(),
        call_data.clone(),
        native_value.to_le_bytes(),
    );

    // Get the updated accounts
    let governance_config_account_updated = schedule_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap()
        .1
        .clone();

    let proposal_pda_account_updated = schedule_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap()
        .1
        .clone();

    let initial_governance_lamports = governance_config_account_updated.lamports;

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
        (setup.gateway_root_pda, gateway_root),
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
        (
            receiver_pubkey,
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
            // Remaining accounts (will be used in execution)
            AccountMeta::new_readonly(GOVERNANCE_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(governance_setup.governance_config, false),
            AccountMeta::new(receiver_pubkey, false),
        ])
        .collect(),
        data: instruction_data,
    };

    // Set clock to after ETA
    let eta_timestamp: i64 = eta.try_into().unwrap_or(1800000000i64);
    let current_timestamp = eta_timestamp + 3600; // 1 hour past ETA
    governance_setup.mollusk.sysvars.clock = Clock {
        slot: 1000,
        epoch_start_timestamp: eta_timestamp,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: current_timestamp,
    };

    let execute_result = governance_setup
        .mollusk
        .process_instruction(&instruction, &accounts);

    assert!(
        !execute_result.program_result.is_err(),
        "Execute withdraw proposal should succeed: {:?}",
        execute_result.program_result
    );

    // Verify the proposal PDA was closed
    let proposal_account_after_execution = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == proposal_pda)
        .unwrap();

    assert_eq!(proposal_account_after_execution.1.data.len(), 0);
    assert_eq!(proposal_account_after_execution.1.lamports, 0);
    assert_eq!(proposal_account_after_execution.1.owner, SYSTEM_PROGRAM_ID);

    // Verify the receiver got the withdrawn amount
    let receiver_account_after = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == receiver_pubkey)
        .unwrap();

    assert_eq!(
        receiver_account_after.1.lamports, withdraw_amount,
        "Receiver should have received the withdrawn amount"
    );

    // Verify governance config lost the withdrawn amount
    let governance_config_after = execute_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == governance_setup.governance_config)
        .unwrap();

    assert!(governance_config_after.1.lamports < initial_governance_lamports);
}

#[test]
fn should_fail_direct_schedule_timelock_proposal_call() {
    let setup = mock_setup_test();
    let chain_hash = [1u8; 32];
    let address_hash = [2u8; 32];
    let minimum_proposal_eta_delay = 3600;

    let governance_config = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        setup.operator.to_bytes(),
    );

    let init_result = initialize_governance(&setup, governance_config);
    assert!(!init_result.program_result.is_err());

    let proposal_hash = [42u8; 32];
    let eta = 1000000000u64;
    let native_value = vec![0u8; 32];
    let target = setup.payer.to_bytes().to_vec();
    let call_data = vec![1, 2, 3, 4];

    let (proposal_pda, _proposal_bump) = Pubkey::find_program_address(
        &[
            axelar_solana_governance_v2::seed_prefixes::PROPOSAL_PDA,
            &proposal_hash,
        ],
        &GOVERNANCE_PROGRAM_ID,
    );

    let instruction_data = {
        use anchor_lang::InstructionData;
        axelar_solana_governance_v2::instruction::ScheduleTimelockProposal {
            proposal_hash,
            eta,
            native_value: native_value.clone(),
            target: target.clone(),
            call_data: call_data.clone(),
        }
        .data()
    };

    let accounts = vec![
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        AccountMeta::new_readonly(setup.governance_config, false), // if an external entity calls it directly
        AccountMeta::new(setup.payer, true),
        AccountMeta::new(proposal_pda, false),
        AccountMeta::new_readonly(setup.event_authority_pda, false),
        AccountMeta::new_readonly(GOVERNANCE_PROGRAM_ID, false),
    ];

    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts,
        data: instruction_data,
    };

    let governance_config_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.governance_config)
        .unwrap()
        .1
        .clone();

    let result = setup.mollusk.process_instruction(
        &instruction,
        &[
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
            (setup.governance_config, governance_config_account),
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
                proposal_pda,
                Account {
                    lamports: 0,
                    data: vec![],
                    owner: SYSTEM_PROGRAM_ID,
                    executable: false,
                    rent_epoch: 0,
                },
            ),
            // for emit cpi
            (
                setup.event_authority_pda,
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
                    lamports: 0,
                    data: vec![],
                    owner: solana_sdk::bpf_loader_upgradeable::id(),
                    executable: true,
                    rent_epoch: 0,
                },
            ),
        ],
    );

    assert!(result.program_result.is_err());
}

use anchor_lang::prelude::ToAccountMetas;
use anchor_lang::AccountDeserialize;
use axelar_solana_governance_v2::state::GovernanceConfig;
use axelar_solana_governance_v2::state::GovernanceConfigInit;
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use axelar_solana_governance_v2_test_fixtures::{
    create_transfer_operatorship_instruction_data, initialize_governance, mock_setup_test,
};
use solana_sdk::account::Account;
use solana_sdk::instruction::Instruction;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

#[test]
fn should_transfer_operatorship() {
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

    let result = initialize_governance(&setup, governance_config.clone());
    assert!(!result.program_result.is_err());

    let new_operator = Pubkey::new_unique();
    let instruction_data = create_transfer_operatorship_instruction_data(new_operator);

    let governance_config_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.governance_config)
        .unwrap()
        .1
        .clone();

    // Set up accounts for transfer operatorship instruction
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
            setup.operator,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (setup.governance_config, governance_config_account),
        // For event CPI
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
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ];

    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: axelar_solana_governance_v2::accounts::TransferOperatorship {
            system_program: SYSTEM_PROGRAM_ID,
            operator_account: Some(setup.operator),
            governance_config: setup.governance_config,
            event_authority: setup.event_authority_pda,
            program: GOVERNANCE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_data,
    };

    let transfer_result = setup.mollusk.process_instruction(&instruction, &accounts);

    assert!(
        !transfer_result.program_result.is_err(),
        "Transfer operatorship should succeed: {:?}",
        transfer_result.program_result
    );

    // Verify the operator was changed
    let updated_governance_config_account = transfer_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.governance_config)
        .unwrap()
        .1
        .clone();

    let updated_config =
        GovernanceConfig::try_deserialize(&mut updated_governance_config_account.data.as_slice())
            .unwrap();

    assert_eq!(
        updated_config.operator,
        new_operator.to_bytes(),
        "Operator should have been updated to the new operator"
    );

    // Original config should remain the same except for operator
    assert_eq!(updated_config.chain_hash, governance_config.chain_hash);
    assert_eq!(updated_config.address_hash, governance_config.address_hash);
    assert_eq!(
        updated_config.minimum_proposal_eta_delay,
        governance_config.minimum_proposal_eta_delay
    );
}

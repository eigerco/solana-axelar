use anchor_lang::{prelude::ProgramError, AccountDeserialize};
use axelar_solana_its_v2::state::InterchainTokenService;
use mollusk_svm::result::Check;
use mollusk_test_utils::setup_mollusk;
use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program, InstructionData, ToAccountMetas,
    },
    solana_sdk::{account::Account, pubkey::Pubkey},
};

// Import helper functions from initialize.rs
mod initialize;
use initialize::init_its_service;

#[test]
fn test_set_pause_status_success() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();
    let payer = upgrade_authority; // Must be upgrade authority
    let payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service first
    let (
        its_root_pda,
        its_root_account,
        _user_roles_pda,
        _user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        payer,
        &payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Verify initial state is unpaused
    let its_data = InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
        .expect("Failed to deserialize ITS data");
    assert_eq!(its_data.paused, false);

    // Now test pausing
    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer,
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: true }.data(),
    };

    let accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
    ];

    let checks = vec![Check::success()];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // Verify the pause status was changed
    let updated_its_account = result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let updated_its_data =
        InterchainTokenService::try_deserialize(&mut updated_its_account.data.as_slice())
            .expect("Failed to deserialize updated ITS data");

    assert_eq!(updated_its_data.paused, true);

    // Test unpausing
    let unpause_ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer,
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: false }.data(),
    };

    let unpause_accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, updated_its_account.clone()),
    ];

    let unpause_result =
        mollusk.process_and_validate_instruction(&unpause_ix, &unpause_accounts, &checks);

    // Verify the pause status was changed back to false
    let final_its_account = unpause_result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let final_its_data =
        InterchainTokenService::try_deserialize(&mut final_its_account.data.as_slice())
            .expect("Failed to deserialize final ITS data");

    assert_eq!(final_its_data.paused, false);
}

#[test]
fn test_set_pause_status_already_paused() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();
    let payer = upgrade_authority;
    let payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service
    let (
        its_root_pda,
        its_root_account,
        _user_roles_pda,
        _user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        payer,
        &payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // First, pause the service
    let pause_ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer,
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: true }.data(),
    };

    let pause_accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
    ];

    let pause_result =
        mollusk.process_and_validate_instruction(&pause_ix, &pause_accounts, &[Check::success()]);
    let paused_its_account = pause_result.get_account(&its_root_pda).unwrap();

    // Now try to pause again (should fail due to constraint)
    let duplicate_pause_ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer,
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: true }.data(),
    };

    let duplicate_pause_accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, paused_its_account.clone()),
    ];

    let checks = vec![Check::err(ProgramError::InvalidArgument)];

    mollusk.process_and_validate_instruction(
        &duplicate_pause_ix,
        &duplicate_pause_accounts,
        &checks,
    );
}

#[test]
fn test_set_pause_status_unauthorized() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();
    let authorized_payer = upgrade_authority;
    let authorized_payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    // Unauthorized user
    let unauthorized_payer = Pubkey::new_unique();
    let unauthorized_payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service with the authorized payer
    let (
        its_root_pda,
        its_root_account,
        _user_roles_pda,
        _user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        authorized_payer,
        &authorized_payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Try to pause with unauthorized user (should fail)
    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer: unauthorized_payer, // Different from upgrade authority
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: true }.data(),
    };

    let accounts = vec![
        (unauthorized_payer, unauthorized_payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
    ];

    let checks = vec![Check::err(ProgramError::InvalidAccountData)];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // Verify the pause status was NOT changed
    let unchanged_its_data =
        InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
            .expect("Failed to deserialize unchanged ITS data");

    assert_eq!(unchanged_its_data.paused, false); // Should still be unpaused
}

#[test]
fn test_set_pause_status_already_unpaused() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();
    let payer = upgrade_authority;
    let payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service (starts unpaused)
    let (
        its_root_pda,
        its_root_account,
        _user_roles_pda,
        _user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        payer,
        &payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Try to unpause when already unpaused (should fail due to constraint)
    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetPauseStatus {
            payer,
            program_data,
            its_root_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetPauseStatus { paused: false }.data(),
    };

    let accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
    ];

    let checks = vec![Check::err(ProgramError::InvalidArgument)];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);
}

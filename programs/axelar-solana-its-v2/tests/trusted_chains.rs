use anchor_lang::{prelude::ProgramError, AccountDeserialize, Discriminator};
use axelar_solana_its_v2::state::{InterchainTokenService, Roles, RolesError, UserRoles};
use mollusk_svm::{program::keyed_account_for_system_program, result::Check};
use mollusk_test_utils::{get_event_authority_and_program_accounts, setup_mollusk};
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
fn test_set_trusted_chain_success() {
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
        user_roles_pda,
        user_roles_account,
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

    // Verify initial state has no trusted chains
    let its_data = InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
        .expect("Failed to deserialize ITS data");
    assert_eq!(its_data.trusted_chains.len(), 0);

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    // Now test adding a trusted chain
    let trusted_chain_name = "ethereum".to_string();

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer,
            user_roles: None,
            program_data: Some(program_data),
            its_root_pda,
            system_program: system_program::ID,
            // Event authority
            event_authority: event_authority,
            // The current program account
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let accounts = vec![
        (payer, payer_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let checks = vec![Check::success()];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // Verify the trusted chain was added
    let updated_its_account = result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let updated_its_data =
        InterchainTokenService::try_deserialize(&mut updated_its_account.data.as_slice())
            .expect("Failed to deserialize updated ITS data");

    assert_eq!(updated_its_data.trusted_chains.len(), 1);
    assert!(updated_its_data
        .trusted_chains
        .contains(&trusted_chain_name));
    assert!(updated_its_data.is_trusted_chain(trusted_chain_name.clone()));
}

#[test]
fn test_set_trusted_chain_operator_success() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    // The payer is the operator
    let init_payer = upgrade_authority;
    let init_payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service first
    let (
        its_root_pda,
        its_root_account,
        user_roles_pda,
        user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        init_payer,
        &init_payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Verify initial state has no trusted chains
    let its_data = InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
        .expect("Failed to deserialize ITS data");
    assert_eq!(its_data.trusted_chains.len(), 0);

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    // Now test adding a trusted chain
    let trusted_chain_name = "ethereum".to_string();

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer: operator,
            user_roles: Some(user_roles_pda),
            program_data: None,
            its_root_pda,
            system_program: system_program::ID,
            // Event authority
            event_authority: event_authority,
            // The current program account
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let accounts = vec![
        (operator, operator_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let checks = vec![Check::success()];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // Verify the trusted chain was added
    let updated_its_account = result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let updated_its_data =
        InterchainTokenService::try_deserialize(&mut updated_its_account.data.as_slice())
            .expect("Failed to deserialize updated ITS data");

    assert_eq!(updated_its_data.trusted_chains.len(), 1);
    assert!(updated_its_data
        .trusted_chains
        .contains(&trusted_chain_name));
    assert!(updated_its_data.is_trusted_chain(trusted_chain_name.clone()));
}

#[test]
fn test_set_trusted_chain_operator_and_upgrade_authority_success() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();

    let operator = upgrade_authority;
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    // The payer is the operator
    let init_payer = upgrade_authority;
    let init_payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service first
    let (
        its_root_pda,
        its_root_account,
        user_roles_pda,
        user_roles_account,
        program_data,
        program_data_account,
    ) = init_its_service(
        &mollusk,
        init_payer,
        &init_payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Verify initial state has no trusted chains
    let its_data = InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
        .expect("Failed to deserialize ITS data");
    assert_eq!(its_data.trusted_chains.len(), 0);

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    // Now test adding a trusted chain
    let trusted_chain_name = "ethereum".to_string();

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer: operator,
            user_roles: Some(user_roles_pda),
            program_data: Some(program_data),
            its_root_pda,
            system_program: system_program::ID,
            // Event authority
            event_authority: event_authority,
            // The current program account
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let accounts = vec![
        (operator, operator_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let checks = vec![Check::success()];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // Verify the trusted chain was added
    let updated_its_account = result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let updated_its_data =
        InterchainTokenService::try_deserialize(&mut updated_its_account.data.as_slice())
            .expect("Failed to deserialize updated ITS data");

    assert_eq!(updated_its_data.trusted_chains.len(), 1);
    assert!(updated_its_data
        .trusted_chains
        .contains(&trusted_chain_name));
    assert!(updated_its_data.is_trusted_chain(trusted_chain_name.clone()));
}

#[test]
fn test_set_trusted_chain_already_exists() {
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
        user_roles_pda,
        user_roles_account,
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

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    let trusted_chain_name = "ethereum".to_string();

    // First, add a trusted chain successfully
    let add_ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer,
            user_roles: None,
            program_data: Some(program_data),
            its_root_pda,
            system_program: system_program::ID,
            event_authority,
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let add_accounts = vec![
        (payer, payer_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account.clone()),
        (program_id, program_account.clone()),
    ];

    let add_result =
        mollusk.process_and_validate_instruction(&add_ix, &add_accounts, &[Check::success()]);
    let updated_its_account = add_result.get_account(&its_root_pda).unwrap();

    // Now try to add the same chain again (should fail)
    let duplicate_add_ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer,
            user_roles: None,
            program_data: Some(program_data),
            its_root_pda,
            system_program: system_program::ID,
            event_authority,
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let duplicate_add_accounts = vec![
        (payer, payer_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, updated_its_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let checks = vec![Check::err(ProgramError::InvalidArgument)];

    mollusk.process_and_validate_instruction(&duplicate_add_ix, &duplicate_add_accounts, &checks);
}

#[test]
fn test_set_trusted_chain_unauthorized() {
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
        user_roles_pda,
        user_roles_account,
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

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    // Try to add trusted chain with unauthorized user (should fail)
    let trusted_chain_name = "ethereum".to_string();

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer: unauthorized_payer, // Different from upgrade authority
            user_roles: None,
            program_data: Some(program_data),
            its_root_pda,
            system_program: system_program::ID,
            event_authority,
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let accounts = vec![
        (unauthorized_payer, unauthorized_payer_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let checks = vec![Check::err(ProgramError::MissingRequiredSignature)];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);
}

#[test]
fn test_set_trusted_chain_missing_operator_role() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();

    let init_payer = upgrade_authority;
    let init_payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // Initialize the ITS service with the authorized payer
    let (
        its_root_pda,
        its_root_account,
        user_roles_pda,
        mut user_roles_account,
        _program_data,
        _program_data_account,
    ) = init_its_service(
        &mollusk,
        init_payer,
        &init_payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    // Try to add trusted chain with unauthorized user (should fail)
    let trusted_chain_name = "ethereum".to_string();

    // Update operator user role to not be OPERATOR
    user_roles_account.data[UserRoles::DISCRIMINATOR.len()] = Roles::MINTER.bits();

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
            payer: operator,
            user_roles: Some(user_roles_pda),
            program_data: None,
            its_root_pda,
            system_program: system_program::ID,
            event_authority,
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::SetTrustedChain {
            chain_name: trusted_chain_name.clone(),
        }
        .data(),
    };

    let accounts = vec![
        (operator, operator_account.clone()),
        (user_roles_pda, user_roles_account.clone()),
        (its_root_pda, its_root_account.clone()),
        keyed_account_for_system_program(),
        (event_authority, event_authority_account),
        (program_id, program_account),
    ];

    let anchor_err: anchor_lang::error::Error = RolesError::MissingOperatorRole.into();
    let checks = vec![Check::err(anchor_err.into())];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);
}

#[test]
fn test_set_multiple_trusted_chains() {
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
        mut its_root_account,
        user_roles_pda,
        user_roles_account,
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

    let trusted_chains = vec![
        "ethereum".to_string(),
        "polygon".to_string(),
        "avalanche".to_string(),
    ];

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    for (i, trusted_chain_name) in trusted_chains.iter().enumerate() {
        let ix = Instruction {
            program_id,
            accounts: axelar_solana_its_v2::accounts::SetTrustedChain {
                payer,
                user_roles: None,
                program_data: Some(program_data),
                its_root_pda,
                system_program: system_program::ID,
                event_authority,
                program: program_id,
            }
            .to_account_metas(None),
            data: axelar_solana_its_v2::instruction::SetTrustedChain {
                chain_name: trusted_chain_name.clone(),
            }
            .data(),
        };

        let accounts = vec![
            (payer, payer_account.clone()),
            (user_roles_pda, user_roles_account.clone()),
            (program_data, program_data_account.clone()),
            (its_root_pda, its_root_account.clone()),
            keyed_account_for_system_program(),
            (event_authority, event_authority_account.clone()),
            (program_id, program_account.clone()),
        ];

        let result = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);

        // Update account for next iteration
        its_root_account = result.get_account(&its_root_pda).unwrap().clone();

        // Verify the chain was added
        let its_data =
            InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
                .expect("Failed to deserialize ITS data");

        assert_eq!(its_data.trusted_chains.len(), i + 1);
        assert!(its_data.trusted_chains.contains(trusted_chain_name));
    }

    // Final verification that all chains are present
    let final_its_data =
        InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
            .expect("Failed to deserialize final ITS data");

    assert_eq!(final_its_data.trusted_chains.len(), 3);
    for chain in &trusted_chains {
        assert!(final_its_data.trusted_chains.contains(chain));
    }
}

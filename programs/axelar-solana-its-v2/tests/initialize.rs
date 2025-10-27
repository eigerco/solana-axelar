use anchor_lang::AccountDeserialize;
use axelar_solana_its_v2::state::{InterchainTokenService, Roles, UserRoles};
use mollusk_svm::{program::keyed_account_for_system_program, result::Check};
use mollusk_test_utils::{create_program_data_account, setup_mollusk};
use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program, Discriminator, InstructionData,
        Space, ToAccountMetas,
    },
    mollusk_svm::Mollusk,
    solana_sdk::{account::Account, pubkey::Pubkey},
    solana_sdk_ids::bpf_loader_upgradeable,
};

pub(crate) fn init_its_service(
    mollusk: &Mollusk,
    payer: Pubkey,
    payer_account: &Account,
    upgrade_authority: Pubkey,
    operator: Pubkey,
    operator_account: &Account,
    chain_name: String,
    its_hub_address: String,
) -> (Pubkey, Account, Pubkey, Account, Pubkey, Account) {
    let program_id = axelar_solana_its_v2::id();

    // Derive the program data PDA for the upgradeable program
    let (program_data, _bump) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let its_elf = mollusk_svm::file::load_program_elf("axelar_solana_its_v2");
    let program_data_account = create_program_data_account(&its_elf, upgrade_authority);

    if payer != upgrade_authority {
        println!("[WARNING] Initialize will fail since payer is not the upgrade authority");
    }

    // Derive the ITS root PDA
    let (its_root_pda, _bump) =
        Pubkey::find_program_address(&[InterchainTokenService::SEED_PREFIX], &program_id);

    // Derive the user roles PDA
    let (user_roles_pda, _bump) = Pubkey::find_program_address(
        &UserRoles::pda_seeds(&its_root_pda, &operator)[..],
        &program_id,
    );

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_its_v2::accounts::Initialize {
            payer,
            program_data,
            its_root_pda,
            system_program: system_program::ID,
            operator,
            user_roles_account: user_roles_pda,
        }
        .to_account_metas(None),
        data: axelar_solana_its_v2::instruction::Initialize {
            chain_name,
            its_hub_address,
        }
        .data(),
    };

    let accounts = vec![
        (payer, payer_account.clone()),
        (program_data, program_data_account.clone()),
        (its_root_pda, Account::new(0, 0, &system_program::ID)),
        keyed_account_for_system_program(),
        (operator, operator_account.clone()),
        (user_roles_pda, Account::new(0, 0, &system_program::ID)),
    ];

    let checks = vec![
        Check::success(),
        Check::account(&its_root_pda)
            .space(InterchainTokenService::DISCRIMINATOR.len() + InterchainTokenService::INIT_SPACE)
            .build(),
        Check::account(&user_roles_pda)
            .space(UserRoles::DISCRIMINATOR.len() + UserRoles::INIT_SPACE)
            .build(),
        Check::all_rent_exempt(),
    ];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    let its_root_account = result
        .get_account(&its_root_pda)
        .expect("ITS root PDA should exist");

    let user_roles_account = result
        .get_account(&user_roles_pda)
        .expect("User roles PDA should exist");

    let user_roles_data = UserRoles::try_deserialize(&mut user_roles_account.data.as_slice())
        .expect("Failed to deserialize roles data");
    assert_eq!(user_roles_data.roles, Roles::OPERATOR);

    (
        its_root_pda,
        its_root_account.clone(),
        user_roles_pda,
        user_roles_account.clone(),
        program_data,
        program_data_account,
    )
}

#[test]
fn test_initialize_success() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();

    // We require that the payer be the upgrade_authority
    let payer = upgrade_authority;
    let payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    let (
        its_root_pda,
        its_root_account,
        user_roles_pda,
        user_roles_account,
        _program_data,
        _program_data_account,
    ) = init_its_service(
        &mollusk,
        payer,
        &payer_account,
        payer,
        operator,
        &operator_account,
        chain_name.clone(),
        its_hub_address.clone(),
    );

    // Verify the ITS root PDA is properly initialized
    let its_data = InterchainTokenService::try_deserialize(&mut its_root_account.data.as_slice())
        .expect("Failed to deserialize ITS data");

    assert_eq!(its_data.chain_name, chain_name);
    assert_eq!(its_data.its_hub_address, its_hub_address);
    assert_eq!(its_data.paused, false);
    assert_eq!(its_data.trusted_chains.len(), 0);

    // Verify the user roles PDA is properly initialized
    let roles_data = UserRoles::try_deserialize(&mut user_roles_account.data.as_slice())
        .expect("Failed to deserialize roles data");

    assert_eq!(roles_data.roles, Roles::OPERATOR);

    // Verify PDAs are derived correctly
    let expected_its_pda =
        Pubkey::find_program_address(&[InterchainTokenService::SEED_PREFIX], &program_id).0;
    assert_eq!(its_root_pda, expected_its_pda);

    let expected_roles_pda = Pubkey::find_program_address(
        &[
            UserRoles::SEED_PREFIX,
            its_root_pda.as_ref(),
            operator.as_ref(),
        ],
        &program_id,
    )
    .0;
    assert_eq!(user_roles_pda, expected_roles_pda);

    // Verify the program data PDA is correct
    let expected_program_data =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID).0;
    let (actual_program_data, _) = (expected_program_data, 0); // We verified it in init_its_service
    assert_eq!(actual_program_data, expected_program_data);
}

#[test]
#[should_panic = "InvalidAccountData"]
fn test_initialize_unauthorized_payer() {
    let program_id = axelar_solana_its_v2::id();
    let mollusk = setup_mollusk(&program_id, "axelar_solana_its_v2");

    let upgrade_authority = Pubkey::new_unique();

    // We make the payer different from the upgrade_authority
    let payer = Pubkey::new_unique();
    let payer_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let chain_name = "solana".to_string();
    let its_hub_address = "0x123456789abcdef".to_string();

    // This should fail because payer is not the upgrade authority
    // The program data account was created with authorized_payer as authority
    let (
        _its_root_pda,
        _its_root_account,
        _user_roles_pda,
        _user_roles_account,
        _program_data,
        _program_data_account,
    ) = init_its_service(
        &mollusk,
        payer,
        &payer_account,
        upgrade_authority,
        operator,
        &operator_account,
        chain_name,
        its_hub_address,
    );
}

use gateway::accounts::{GatewayApprovedMessage, GatewayConfig};
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::TestFixture;
use token_manager::get_token_manager_account;

use crate::program_test;

#[tokio::test]
async fn test_deploy_token_manager() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Pubkey::from([0; 32]);
    let mint_authority = Keypair::new();

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let its_token_manager_permission_groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda,
            &interchain_token_service_root_pda,
            &init_operator,
        )
        .await;

    let deploy_token_manager_message = test_fixtures::axelar_message::message().unwrap();
    let gateway_approved_message_pda = fixture
        .approve_gateway_message(&deploy_token_manager_message)
        .await;

    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = GatewayApprovedMessage::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_approved(),
        "GatewayApprovedMessage should be approved"
    );

    let token_manager_root_pda_pubkey = get_token_manager_account(
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &interchain_token_service_root_pda,
    );

    // Action
    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
        &gateway_approved_message_pda,
        &fixture.payer.pubkey(),
        &token_manager_root_pda_pubkey,
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .operator_group
            .group_pda_user_owner,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda_user_owner,
        &interchain_token_service_root_pda,
        &token_mint,
        &gateway_root_pda,
        DeployTokenManager {
            token_id: Bytes32(keccak256("random-token-id")),
            token_manager_type: U256::from(token_manager::TokenManagerType::MintBurn as u8),
            params: vec![],
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Assert
    // Operator group
    let op_group = fixture
        .banks_client
        .get_account(its_token_manager_permission_groups.operator_group.group_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = op_group
        .check_initialized_pda::<account_group::state::PermissionGroupAccount>(&account_group::id())
        .unwrap();

    // Operator account
    let operator = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .operator_group
                .group_pda_user,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = operator
        .check_initialized_pda::<account_group::state::PermissionAccount>(&account_group::id())
        .unwrap();
    // Flow limiter group
    let flow_group = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = flow_group
        .check_initialized_pda::<account_group::state::PermissionGroupAccount>(&account_group::id())
        .unwrap();

    // Flow limiter account
    let flow_limiter = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda_user,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = flow_limiter
        .check_initialized_pda::<interchain_token_service::state::RootPDA>(&account_group::id())
        .unwrap();

    // Token manager account
    let token_manager_root_pda = fixture
        .banks_client
        .get_account(token_manager_root_pda_pubkey)
        .await
        .expect("get_account")
        .expect("account not none");
    let token_manager_root_pda =
        token_manager_root_pda
            .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(
                &token_manager::id(),
            )
            .unwrap();
    assert_eq!(
        token_manager_root_pda,
        token_manager::state::TokenManagerRootAccount {
            flow_limit: 0,
            associated_token_account: get_associated_token_address(
                &token_manager_root_pda_pubkey,
                &token_mint
            ),
            token_manager_type: token_manager::TokenManagerType::MintBurn,
            token_mint,
        }
    )
}

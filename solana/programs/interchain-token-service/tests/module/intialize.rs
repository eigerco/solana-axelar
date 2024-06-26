use interchain_token_service::get_interchain_token_service_root_pda;
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_init_root_pda_interchain_token_service() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
        .await;
    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    // Action
    let ix = interchain_token_service::instruction::build_initialize_instruction(
        &fixture.payer.pubkey(),
        &interchain_token_service_root_pda,
        &gateway_root_pda,
        &gas_service_root_pda,
    )
    .unwrap();
    let blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Assert
    let interchain_token_service_root_pda = fixture
        .banks_client
        .get_account(interchain_token_service_root_pda)
        .await
        .expect("get_account")
        .expect("account not none");

    let its_root_pda = interchain_token_service_root_pda
        .check_initialized_pda::<interchain_token_service::state::RootPDA>(
            &interchain_token_service::id(),
        )
        .unwrap();
    assert_eq!(
        its_root_pda,
        interchain_token_service::state::RootPDA::new(its_root_pda.bump_seed)
    );

    let its_root_pda =
        interchain_token_service::state::RootPDA::unpack(&interchain_token_service_root_pda.data)
            .unwrap();
    assert_eq!(
        its_root_pda,
        interchain_token_service::state::RootPDA::new(its_root_pda.bump_seed)
    );
}

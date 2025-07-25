use axelar_solana_gas_service_events::events::{GasServiceEvent, NativeGasAddedEvent};
use axelar_solana_gateway_test_fixtures::{base::TestFixture, gas_service::get_gas_service_events};
use gateway_event_stack::ProgramInvocationState;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_add_native_gas() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    let payer = Keypair::new();
    test_fixture
        .fund_account(&payer.pubkey(), 1_000_000_000)
        .await;
    let payer_balance_before = test_fixture
        .try_get_account_no_checks(&payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let config_pda_balance_before = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Action
    let refund_address = Pubkey::new_unique();
    let gas_amount = 1_000_000;
    let tx_hash = [42; 64];
    let log_index = 1232;
    let ix = axelar_solana_gas_service::instructions::add_native_gas_instruction(
        &payer.pubkey(),
        tx_hash,
        log_index,
        gas_amount,
        refund_address,
    )
    .unwrap();

    let res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // pays for gas deduction
                &payer,
            ],
        )
        .await
        .unwrap();

    // assert event
    let emitted_events = get_gas_service_events(&res).into_iter().next().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GasServiceEvent::NativeGasAdded(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    assert_eq!(
        emitted_event,
        &NativeGasAddedEvent {
            config_pda: gas_utils.config_pda,
            tx_hash,
            log_index,
            refund_address,
            gas_fee_amount: gas_amount,
        }
    );

    // assert that SOL gets transferred
    let payer_balance_after = test_fixture
        .try_get_account_no_checks(&payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let config_pda_balance_after = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        config_pda_balance_after,
        config_pda_balance_before + gas_amount
    );
    assert_eq!(payer_balance_after, payer_balance_before - gas_amount);
}

#[tokio::test]
async fn fails_if_payer_not_signer() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    let payer = Keypair::new();
    test_fixture
        .fund_account(&payer.pubkey(), 1_000_000_000)
        .await;

    // Action
    let refund_address = Pubkey::new_unique();
    let gas_amount = 1_000_000;
    let tx_hash = [42; 64];
    let log_index = 1232;
    let mut ix = axelar_solana_gas_service::instructions::add_native_gas_instruction(
        &payer.pubkey(),
        tx_hash,
        log_index,
        gas_amount,
        refund_address,
    )
    .unwrap();
    ix.accounts[0].is_signer = false;

    let res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
            ],
        )
        .await;
    assert!(res.is_err());
}

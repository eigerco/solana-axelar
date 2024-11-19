use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::IxBuilder;
use axelar_solana_governance::state::GovernanceConfig;
use rkyv::Deserialize;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::TestFixture;

use crate::fixtures::operator_keypair;
use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, default_proposal_eta, events,
    gmp_sample_metadata, init_contract_with_operator, program_test, setup_programs,
};

#[tokio::test]
async fn test_operator_transfer_can_happen_being_operator_signer() {
    // Get the operator key pair;
    let operator = operator_keypair();

    let mut fixture = TestFixture::new(program_test()).await;

    // Setup gov module (initialize contract)
    let (config_pda, _) =
        init_contract_with_operator(&mut fixture, operator_keypair().pubkey().to_bytes())
            .await
            .unwrap();

    let new_operator = Pubkey::new_unique();

    let ix = IxBuilder::new()
        .transfer_operatorship(
            &fixture.payer.pubkey(),
            &operator.pubkey(),
            &config_pda,
            &new_operator,
        )
        .build();

    let res = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[operator.insecure_clone(), fixture.payer.insecure_clone()],
        )
        .await;
    assert!(res.result.is_ok());

    // Check the new operator was properly set
    let config = fixture
        .banks_client
        .get_account(config_pda)
        .await
        .unwrap()
        .unwrap();
    let config = config
        .check_rkyv_initialized_pda::<GovernanceConfig>(&axelar_solana_governance::id())
        .unwrap();
    assert_eq!(new_operator.to_bytes(), config.operator);

    // Assert event was emitted
    let mut emitted_events = events(&res);
    assert_eq!(emitted_events.len(), 1);
    let expected_event = operatorship_transferred_event(&operator.pubkey(), &new_operator);
    let got_event: GovernanceEvent = emitted_events
        .pop()
        .unwrap()
        .parse()
        .deserialize(&mut rkyv::Infallible)
        .unwrap();
    assert_eq!(expected_event, got_event);
}

const fn operatorship_transferred_event(
    old_operator: &Pubkey,
    new_operator: &Pubkey,
) -> GovernanceEvent {
    GovernanceEvent::OperatorshipTransferred {
        old_operator: old_operator.to_bytes(),
        new_operator: new_operator.to_bytes(),
    }
}

#[tokio::test]
async fn test_error_is_emitted_when_no_required_signers() {
    // Not current operator
    let operator = Keypair::new();

    let mut fixture = TestFixture::new(program_test()).await;

    // Setup gov module (initialize contract)
    let (config_pda, _) =
        init_contract_with_operator(&mut fixture, operator_keypair().pubkey().to_bytes())
            .await
            .unwrap();

    let ix = IxBuilder::new()
        .transfer_operatorship(
            &fixture.payer.pubkey(),
            &operator.pubkey(),
            &config_pda,
            &Pubkey::new_unique(),
        )
        .build();

    let res = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[fixture.payer.insecure_clone(), operator], // Not current operator
        )
        .await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Operator account must sign the transaction");
}

#[tokio::test]
async fn test_can_change_operator_via_gmp_proposal() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let new_operator = Pubkey::new_unique();

    let ix_builder = IxBuilder::new().builder_for_operatorship_transfership(
        &sol_integration.fixture.payer.pubkey(),
        &config_pda,
        &operator_keypair().pubkey(),
        &new_operator,
        default_proposal_eta(),
    );

    // Send the GMP instruction for scheduling the proposal, that later will target
    // the governance module itself to change the operator.
    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Move time forward to the proposal ETA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Send the proposal execution instruction

    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Check the new operator was properly set
    let config = sol_integration
        .fixture
        .banks_client
        .get_account(config_pda)
        .await
        .unwrap()
        .unwrap();
    let config = config
        .check_rkyv_initialized_pda::<GovernanceConfig>(&axelar_solana_governance::id())
        .unwrap();
    assert_eq!(new_operator.to_bytes(), config.operator);
}
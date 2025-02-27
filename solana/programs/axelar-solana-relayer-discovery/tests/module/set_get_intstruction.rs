use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_relayer_discovery::state::RelayerExecutionInfo;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};

#[tokio::test]
async fn test_set_instruction() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;

    let destination_address = Pubkey::new_unique();
    let execution_info = RelayerExecutionInfo::Query(Instruction {
        accounts: vec![],
        program_id: destination_address,
        data: vec![],
    });

    test_fixture
        .send_tx(&[
            axelar_solana_relayer_discovery::instructions::set_instruction(&test_fixture.payer.pubkey(), destination_address, execution_info).unwrap(),
        ])
        .await;
}
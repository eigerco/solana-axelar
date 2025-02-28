use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_relayer_discovery::state::RelayerExecutionInfo;
use borsh::BorshDeserialize;
use gateway_event_stack::decode_base64;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};

#[tokio::test]
async fn test_set_instruction() {
    
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;

    test_fixture.deploy_relayer_discovery().await;

    let destination_address = Pubkey::new_unique();
    let execution_info = RelayerExecutionInfo::Query(Instruction {
        accounts: vec![],
        program_id: destination_address,
        data: vec![1, 2, 3],
    });

    let ix = axelar_solana_relayer_discovery::instructions::set_instruction(&test_fixture.payer.pubkey(), destination_address, execution_info.clone()).unwrap();
    test_fixture
        .send_tx(&[
            ix,
        ])
        .await;

    let ix = axelar_solana_relayer_discovery::instructions::get_instruction(destination_address).unwrap();
    let response = test_fixture
        .send_tx(&[
            ix,
        ])
        .await;
    
    let response = response.unwrap();
    let metadata = response.metadata.unwrap();
    let logs = metadata.log_messages.as_slice();
    let program_return = logs.iter().find(|log| {
        (*log).starts_with("Program return: ")
    }).unwrap();
    
    let mut iter = program_return
        .trim()
        .trim_start_matches("Program data: ")
        .split(' ')
        .filter_map(decode_base64);
    let data = iter.next().unwrap();

    let execution_info_read = RelayerExecutionInfo::try_from_slice(&data).unwrap();
    
    assert_eq!(execution_info, execution_info_read);
}

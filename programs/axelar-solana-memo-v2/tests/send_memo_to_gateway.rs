#![cfg(test)]
#![allow(clippy::str_to_string, clippy::indexing_slicing)]
use anchor_lang::{InstructionData, ToAccountMetas};
use axelar_solana_gateway_v2::ID as GATEWAY_PROGRAM_ID;
use axelar_solana_gateway_v2_test_fixtures::{initialize_gateway, setup_test_with_real_signers};
use axelar_solana_memo_v2::ID as MEMO_PROGRAM_ID;
use solana_sdk::{
    account::Account, instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
    system_program::ID as SYSTEM_PROGRAM_ID,
};

#[test]
#[allow(clippy::too_many_lines)]
#[allow(clippy::non_ascii_literal)]
fn test_send_memo_to_gateway() {
    // Step 0: Example payload
    let memo_string = "🐪🐪🐪🐪";

    // Step 1: Setup test with real signers
    let (mut setup, _verifier_leaves, _verifier_merkle_tree, _secret_key_1, _secret_key_2) =
        setup_test_with_real_signers();

    // Add the memo program to the Mollusk instance
    setup.mollusk.add_program(
        &MEMO_PROGRAM_ID,
        "../../target/deploy/axelar_solana_memo_v2",
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    // Step 2: Initialize gateway
    let init_result = initialize_gateway(&setup);

    let gateway_root_account = init_result.get_account(&setup.gateway_root_pda).unwrap();

    // Step 3: Send memo
    let send_memo_ix = axelar_solana_memo_v2::instruction::SendMemo {
        destination_chain: "ethereum".to_owned(),
        destination_address: "0xDestinationAddress".to_owned(),
        memo: memo_string.to_owned(),
    };
    let (signing_pda, _signing_pda_bump) = Pubkey::find_program_address(
        &[axelar_solana_gateway_v2::seed_prefixes::CALL_CONTRACT_SIGNING_SEED],
        &MEMO_PROGRAM_ID,
    );

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let send_memo_accounts = axelar_solana_memo_v2::accounts::SendMemo {
        memo_program: MEMO_PROGRAM_ID,
        signing_pda,
        gateway_root_pda: setup.gateway_root_pda,
        gateway_event_authority,
        gateway_program: GATEWAY_PROGRAM_ID,
    }
    .to_account_metas(None);

    let send_memo_instruction = Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: send_memo_accounts,
        data: send_memo_ix.data(),
    };
    let send_memo_accounts = vec![
        (
            MEMO_PROGRAM_ID,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            signing_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: MEMO_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (setup.gateway_root_pda, gateway_root_account.clone()),
        (
            gateway_event_authority,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            GATEWAY_PROGRAM_ID,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ];

    let result = setup
        .mollusk
        .process_instruction(&send_memo_instruction, &send_memo_accounts);

    assert!(result.program_result.is_ok());

    // TODO test event cpi
}

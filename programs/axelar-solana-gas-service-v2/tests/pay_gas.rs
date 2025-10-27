#![cfg(test)]
#![allow(clippy::str_to_string)]
use mollusk_svm::{program::keyed_account_for_system_program, result::Check};
use mollusk_test_utils::get_event_authority_and_program_accounts;
use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program, InstructionData, ToAccountMetas,
    },
    solana_sdk::{account::Account, pubkey::Pubkey},
};
mod initialize;
use initialize::{init_gas_service, setup_mollusk, setup_operator};

#[test]
fn test_pay_native_contract_call() {
    // Setup

    let program_id = axelar_solana_gas_service_v2::id();
    let mut mollusk = setup_mollusk(&program_id, "axelar_solana_gas_service_v2");

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let (operator_pda, operator_pda_account) =
        setup_operator(&mut mollusk, operator, &operator_account);

    let (treasury, treasury_pda) = init_gas_service(
        &mollusk,
        operator,
        &operator_account,
        operator_pda,
        &operator_pda_account,
    );

    // Instruction

    let payer = Pubkey::new_unique();
    let payer_balance = 1_000_000_000u64; // 1 SOL
    let payer_account = Account::new(payer_balance, 0, &system_program::ID);

    let gas_fee_amount = 300_000_000u64; // 0.3 SOL
    let refund_address = Pubkey::new_unique();

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_gas_service_v2::accounts::PayGas {
            sender: payer,
            treasury,
            system_program: system_program::ID,
            event_authority,
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_gas_service_v2::instruction::PayGas {
            destination_chain: "chain".to_string(),
            destination_address: "address".to_string(),
            payload_hash: [0u8; 32],
            amount: gas_fee_amount,
            refund_address,
        }
        .data(),
    };

    let accounts = vec![
        (payer, payer_account.clone()),
        (treasury, treasury_pda.clone()),
        keyed_account_for_system_program(),
        // Event authority
        (event_authority, event_authority_account),
        // Current program account (executable)
        (program_id, program_account),
    ];

    // Checks

    let checks = vec![
        Check::success(),
        // Balance subtracted
        Check::account(&payer)
            .lamports(payer_balance - gas_fee_amount)
            .build(),
        // Balance added
        Check::account(&treasury)
            .lamports(treasury_pda.lamports + gas_fee_amount)
            .build(),
    ];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // TODO(v2) check for CPI event emission
}

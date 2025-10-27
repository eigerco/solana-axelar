#![cfg(test)]
use mollusk_svm::result::Check;
use mollusk_test_utils::get_event_authority_and_program_accounts;
use solana_sdk::account::WritableAccount;
use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program, InstructionData, ToAccountMetas,
    },
    solana_sdk::{account::Account, pubkey::Pubkey},
};
mod initialize;
use initialize::{init_gas_service, setup_mollusk, setup_operator};

#[test]
fn test_refund_native_fees() {
    // Setup

    let program_id = axelar_solana_gas_service_v2::id();
    let mut mollusk = setup_mollusk(&program_id, "axelar_solana_gas_service_v2");

    let operator = Pubkey::new_unique();
    let operator_account = Account::new(1_000_000_000, 0, &system_program::ID);

    let (operator_pda, operator_pda_account) =
        setup_operator(&mut mollusk, operator, &operator_account);

    let (treasury, mut treasury_pda) = init_gas_service(
        &mollusk,
        operator,
        &operator_account,
        operator_pda,
        &operator_pda_account,
    );

    let treasury_balance = 10_000_000_000u64; // 10 SOL
    treasury_pda
        .checked_add_lamports(treasury_balance)
        .expect("Failed to add lamports to treasury");

    // Instruction

    let receiver = Pubkey::new_unique();
    let receiver_balance = 1_000_000_000u64; // 1 SOL
    let receiver_account = Account::new(receiver_balance, 0, &system_program::ID);

    let message_id = "tx-sig-2.1".to_owned();
    let fees = 500_000_000u64; // 0.5 SOL

    let (event_authority, event_authority_account, program_account) =
        get_event_authority_and_program_accounts(&program_id);

    let ix = Instruction {
        program_id,
        accounts: axelar_solana_gas_service_v2::accounts::RefundFees {
            operator,
            operator_pda,
            receiver,
            treasury,
            // Event authority
            event_authority,
            // The current program account
            program: program_id,
        }
        .to_account_metas(None),
        data: axelar_solana_gas_service_v2::instruction::RefundFees {
            message_id,
            amount: fees,
        }
        .data(),
    };

    let accounts = vec![
        (operator, operator_account.clone()),
        (operator_pda, operator_pda_account.clone()),
        (receiver, receiver_account.clone()),
        (treasury, treasury_pda.clone()),
        // Event authority
        (event_authority, event_authority_account),
        // Current program account (executable)
        (program_id, program_account),
    ];

    // Checks

    let checks = vec![
        Check::success(),
        // Balance added
        Check::account(&receiver)
            .lamports(receiver_balance + fees)
            .build(),
        // Balance subtracted
        Check::account(&treasury)
            .lamports(treasury_pda.lamports - fees)
            .build(),
    ];

    mollusk.process_and_validate_instruction(&ix, &accounts, &checks);

    // TODO(v2) check for CPI event emission
}

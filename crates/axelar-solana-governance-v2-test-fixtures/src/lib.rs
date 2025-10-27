use anchor_lang::prelude::{AccountMeta, ToAccountMetas, UpgradeableLoaderState};
use anchor_lang::InstructionData;
use axelar_solana_gateway_v2::Message;
use axelar_solana_gateway_v2::{
    seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED, IncomingMessage, ID as GATEWAY_PROGRAM_ID,
};
use axelar_solana_governance::seed_prefixes;
use axelar_solana_governance::{
    processor::gmp::payload_conversions,
    state::proposal::ExecutableProposal as ExecutableProposalV1,
};
use axelar_solana_governance_v2::{
    ExecuteProposalCallData, ExecuteProposalData, GovernanceConfigInit, GovernanceConfigUpdate,
    SolanaAccountMetadata, ID as GOVERNANCE_PROGRAM_ID,
};
use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_sdk::{
    account::Account, instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
    system_program::ID as SYSTEM_PROGRAM_ID,
};

mod gmp;
pub use gmp::*;

pub struct TestSetup {
    pub mollusk: Mollusk,
    pub payer: Pubkey,
    pub upgrade_authority: Pubkey,
    pub operator: Pubkey,
    pub governance_config: Pubkey,
    pub governance_config_bump: u8,
    pub program_data_pda: Pubkey,
    pub event_authority_pda: Pubkey,
    pub event_authority_bump: u8,
}

pub fn mock_setup_test() -> TestSetup {
    let mollusk = Mollusk::new(
        &GOVERNANCE_PROGRAM_ID,
        "../../target/deploy/axelar_solana_governance_v2",
    );

    let payer = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let operator = Pubkey::new_unique();

    // Derive PDAs
    let (governance_config, governance_config_bump) =
        Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &GOVERNANCE_PROGRAM_ID);

    let (program_data_pda, _) = Pubkey::find_program_address(
        &[GOVERNANCE_PROGRAM_ID.as_ref()],
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    let (event_authority_pda, event_authority_bump) =
        Pubkey::find_program_address(&[b"__event_authority"], &GOVERNANCE_PROGRAM_ID);

    TestSetup {
        mollusk,
        payer,
        upgrade_authority,
        operator,
        governance_config,
        governance_config_bump,
        program_data_pda,
        event_authority_pda,
        event_authority_bump,
    }
}

pub fn initialize_governance(setup: &TestSetup, params: GovernanceConfigInit) -> InstructionResult {
    let instruction_data =
        axelar_solana_governance_v2::instruction::InitializeConfig { params }.data();

    let program_data_state = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(setup.upgrade_authority),
    };

    let serialized_program_data = bincode::serialize(&program_data_state).unwrap();

    let accounts = vec![
        (
            setup.payer,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.upgrade_authority,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.program_data_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: serialized_program_data,
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.governance_config,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            SYSTEM_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ];

    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(setup.upgrade_authority, true),
            AccountMeta::new_readonly(setup.program_data_pda, false),
            AccountMeta::new(setup.governance_config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn update_config(
    setup: &TestSetup,
    params: GovernanceConfigUpdate,
    governance_config_data: Vec<u8>,
) -> InstructionResult {
    let instruction_data = axelar_solana_governance_v2::instruction::UpdateConfig { params }.data();

    let accounts = vec![
        (
            setup.operator,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.governance_config,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: governance_config_data,
                owner: GOVERNANCE_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
    ];

    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.operator, true),
            AccountMeta::new(setup.governance_config, false),
        ],
        data: instruction_data,
    };

    setup.mollusk.process_instruction(&instruction, &accounts)
}

#[allow(clippy::too_many_lines)]
pub fn process_gmp_helper(
    setup: &TestSetup,
    message: Message,
    payload: Vec<u8>,
    context: GmpContext,
) -> InstructionResult {
    let instruction_data = axelar_solana_governance_v2::instruction::ProcessGmp {
        message: message.clone(),
        payload: payload.clone(),
    }
    .data();

    let accounts = vec![
        (
            context.incoming_message.pubkey,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: context.incoming_message.data,
                owner: GATEWAY_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            context.signing_pda.pubkey,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            context.gateway_root_pda.pubkey,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: context.gateway_root_pda.data,
                owner: GATEWAY_PROGRAM_ID,
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
        (
            context.event_authority_pda_governance.pubkey,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            GOVERNANCE_PROGRAM_ID,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            context.event_authority_pda.pubkey,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        // GMP Accounts
        (
            solana_sdk::system_program::ID,
            Account {
                lamports: 0,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            setup.governance_config,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: context.governance_config.data,
                owner: GOVERNANCE_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.payer,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            context.proposal.pubkey,
            Account {
                lamports: 0,
                data: context.proposal.data,
                owner: context.proposal.owner,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            context.operator_proposal.pubkey,
            Account {
                lamports: 0,
                data: context.operator_proposal.data,
                owner: context.operator_proposal.owner,
                executable: false,
                rent_epoch: 0,
            },
        ),
    ];

    // Updated instruction accounts:
    let instruction = Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: axelar_solana_governance_v2::accounts::ProcessGmp {
            executable: axelar_solana_governance_v2::accounts::AxelarExecuteAccounts {
                incoming_message_pda: context.incoming_message.pubkey,
                signing_pda: context.signing_pda.pubkey,
                gateway_root_pda: context.gateway_root_pda.pubkey,
                axelar_gateway_program: GATEWAY_PROGRAM_ID,
                event_authority: context.event_authority_pda.pubkey,
            },
            system_program: solana_sdk::system_program::ID,
            payer: setup.payer,
            governance_config: setup.governance_config,
            proposal_pda: context.proposal.pubkey,
            operator_proposal_pda: context.operator_proposal.pubkey,
            governance_event_authority: context.event_authority_pda_governance.pubkey,
            axelar_governance_program: GOVERNANCE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_data,
    };

    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn get_memo_instruction_data(
    memo: String,
    value_receiver: SolanaAccountMetadata,
) -> ExecuteProposalCallData {
    let memo_instruction_data =
        axelar_solana_memo_v2::instruction::EmitMemo { message: memo }.data();

    let (governance_config_pda, _) =
        Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &GOVERNANCE_PROGRAM_ID);

    let governance_config_pda_metadata = SolanaAccountMetadata {
        pubkey: governance_config_pda.to_bytes(),
        is_signer: true,
        is_writable: false,
    };

    let solana_accounts = vec![value_receiver.clone(), governance_config_pda_metadata];

    axelar_solana_governance_v2::state::proposal::ExecuteProposalCallData {
        solana_accounts,
        solana_native_value_receiver_account: Some(value_receiver),
        call_data: memo_instruction_data,
    }
}

pub fn get_withdraw_tokens_instruction_data(
    withdraw_amount: u64,
    receiver: Pubkey,
    governance_config_pda: [u8; 32],
) -> ExecuteProposalCallData {
    let withdraw_instruction_data = axelar_solana_governance_v2::instruction::WithdrawTokens {
        amount: withdraw_amount,
    }
    .data();

    // The withdraw_tokens instruction expects exactly 3 accounts matching the WithdrawTokens struct:
    // 1. system_program: Program<'info, System>
    // 2. governance_config: Account<'info, GovernanceConfig>
    // 3. receiver: AccountInfo<'info> (mut)
    let solana_accounts = vec![
        SolanaAccountMetadata {
            pubkey: SYSTEM_PROGRAM_ID.to_bytes(),
            is_signer: false,
            is_writable: false,
        },
        SolanaAccountMetadata {
            pubkey: governance_config_pda,
            is_signer: true,
            is_writable: true,
        },
        SolanaAccountMetadata {
            pubkey: receiver.to_bytes(),
            is_signer: false,
            is_writable: true,
        },
    ];

    ExecuteProposalCallData {
        solana_accounts,
        solana_native_value_receiver_account: None,
        call_data: withdraw_instruction_data,
    }
}

pub fn extract_proposal_hash_unchecked(payload: &[u8]) -> [u8; 32] {
    let cmd_payload = payload_conversions::decode_payload(payload).unwrap();
    let target_bytes: [u8; 32] = cmd_payload.target.to_vec().try_into().unwrap();
    let target = Pubkey::from(target_bytes);
    let execute_proposal_call_data =
        payload_conversions::decode_payload_call_data(&cmd_payload.call_data).unwrap();

    ExecutableProposalV1::calculate_hash(
        &target,
        &execute_proposal_call_data,
        &cmd_payload.native_value.to_le_bytes(),
    )
}

pub fn create_signing_pda_from_message(
    message: &Message,
    incoming_message: &IncomingMessage,
) -> Pubkey {
    let command_id = message.command_id();

    Pubkey::create_program_address(
        &[
            VALIDATE_MESSAGE_SIGNING_SEED,
            command_id.as_ref(),
            &[incoming_message.signing_pda_bump],
        ],
        &GOVERNANCE_PROGRAM_ID,
    )
    .unwrap()
}

pub fn create_proposal_pda(proposal_hash: &[u8]) -> Pubkey {
    Pubkey::find_program_address(
        &[seed_prefixes::PROPOSAL_PDA, proposal_hash],
        &GOVERNANCE_PROGRAM_ID,
    )
    .0
}

pub fn create_operator_proposal_pda(proposal_hash: &[u8]) -> Pubkey {
    Pubkey::find_program_address(
        &[seed_prefixes::OPERATOR_MANAGED_PROPOSAL, proposal_hash],
        &GOVERNANCE_PROGRAM_ID,
    )
    .0
}

pub fn create_governance_program_data_pda() -> Pubkey {
    Pubkey::find_program_address(
        &[GOVERNANCE_PROGRAM_ID.as_ref()],
        &solana_sdk::bpf_loader_upgradeable::id(),
    )
    .0
}

pub fn create_governance_event_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"__event_authority"], &GOVERNANCE_PROGRAM_ID)
}

pub fn create_governance_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &GOVERNANCE_PROGRAM_ID)
}

pub fn create_gateway_event_authority_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID).0
}

pub fn create_execute_proposal_instruction_data(
    target_address: [u8; 32],
    call_data: ExecuteProposalCallData,
    native_value: [u8; 32],
) -> Vec<u8> {
    let execute_proposal_data = ExecuteProposalData {
        target_address,
        call_data,
        native_value,
    };

    axelar_solana_governance_v2::instruction::ExecuteTimelockProposal {
        execute_proposal_data,
    }
    .data()
}

pub fn create_execute_operator_proposal_instruction_data(
    target_address: [u8; 32],
    call_data: ExecuteProposalCallData,
    native_value: [u8; 32],
) -> Vec<u8> {
    let execute_proposal_data = ExecuteProposalData {
        target_address,
        call_data,
        native_value,
    };

    axelar_solana_governance_v2::instruction::ExecuteOperatorProposal {
        execute_proposal_data,
    }
    .data()
}

pub fn create_transfer_operatorship_instruction_data(new_operator: Pubkey) -> Vec<u8> {
    axelar_solana_governance_v2::instruction::TransferOperatorship {
        new_operator: new_operator.to_bytes(),
    }
    .data()
}

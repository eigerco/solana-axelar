#![allow(clippy::too_many_arguments)]

use anchor_lang::{
    prelude::UpgradeableLoaderState, solana_program, AccountDeserialize, InstructionData,
    ToAccountMetas,
};
use axelar_solana_encoding::{hasher::SolanaSyscallHasher, rs_merkle::MerkleTree};
use axelar_solana_gateway_v2::seed_prefixes::{
    self, CALL_CONTRACT_SIGNING_SEED, GATEWAY_SEED, VERIFIER_SET_TRACKER_SEED,
};
use axelar_solana_gateway_v2::{
    state::config::{InitialVerifierSet, InitializeConfigParams},
    u256::U256,
    MerkleisedMessage, PublicKey, ID as GATEWAY_PROGRAM_ID,
};
use axelar_solana_gateway_v2::{
    CrossChainId, IncomingMessage, Message, MessageLeaf, SigningVerifierSetInfo, VerifierSetLeaf,
};
use libsecp256k1::SecretKey;
use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    system_program::ID as SYSTEM_PROGRAM_ID,
};

pub struct TestSetup {
    pub mollusk: Mollusk,
    pub payer: Pubkey,
    pub upgrade_authority: Pubkey,
    pub operator: Pubkey,
    pub gateway_root_pda: Pubkey,
    pub gateway_bump: u8,
    pub program_data_pda: Pubkey,
    pub verifier_set_tracker_pda: Pubkey,
    pub verifier_bump: u8,
    pub verifier_set_hash: [u8; 32],
    pub domain_separator: [u8; 32],
    pub minimum_rotation_delay: u64,
    pub epoch: U256,
    pub previous_verifier_retention: U256,
    pub gateway_caller_pda: Option<Pubkey>,
    pub gateway_caller_bump: Option<u8>,
    pub event_authority_pda: Option<Pubkey>,
    pub event_authority_bump: Option<u8>,
}

pub fn mock_setup_test(gateway_caller_program_id: Option<Pubkey>) -> TestSetup {
    let mollusk = Mollusk::new(
        &GATEWAY_PROGRAM_ID,
        "../../target/deploy/axelar_solana_gateway_v2",
    );

    let payer = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let operator = Pubkey::new_unique();

    // dummy values
    let verifier_set_hash = [1u8; 32];
    let epoch = U256::from(1);
    let previous_verifier_retention = U256::from(5);
    let domain_separator = [2u8; 32];
    let minimum_rotation_delay = 3600;

    // Derive PDAs
    let (gateway_root_pda, gateway_bump) =
        Pubkey::find_program_address(&[GATEWAY_SEED], &GATEWAY_PROGRAM_ID);

    let (program_data_pda, _) = Pubkey::find_program_address(
        &[GATEWAY_PROGRAM_ID.as_ref()],
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    let (verifier_set_tracker_pda, verifier_bump) = Pubkey::find_program_address(
        &[VERIFIER_SET_TRACKER_SEED, &verifier_set_hash],
        &GATEWAY_PROGRAM_ID,
    );

    match gateway_caller_program_id {
        Some(program_id) => {
            // Derive PDAs specific to memo program
            let (gateway_caller_pda, gateway_caller_bump) =
                Pubkey::find_program_address(&[CALL_CONTRACT_SIGNING_SEED], &program_id);

            let (event_authority_pda, event_authority_bump) =
                Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

            TestSetup {
                mollusk,
                payer,
                upgrade_authority,
                operator,
                gateway_root_pda,
                gateway_bump,
                program_data_pda,
                verifier_set_tracker_pda,
                verifier_bump,
                verifier_set_hash,
                domain_separator,
                minimum_rotation_delay,
                epoch,
                previous_verifier_retention,
                gateway_caller_pda: Some(gateway_caller_pda),
                gateway_caller_bump: Some(gateway_caller_bump),
                event_authority_pda: Some(event_authority_pda),
                event_authority_bump: Some(event_authority_bump),
            }
        }
        None => TestSetup {
            mollusk,
            payer,
            upgrade_authority,
            operator,
            gateway_root_pda,
            gateway_bump,
            program_data_pda,
            verifier_set_tracker_pda,
            verifier_bump,
            verifier_set_hash,
            domain_separator,
            minimum_rotation_delay,
            epoch,
            previous_verifier_retention,
            gateway_caller_pda: None,
            gateway_caller_bump: None,
            event_authority_pda: None,
            event_authority_bump: None,
        },
    }
}

pub fn setup_test_with_real_signers() -> (
    TestSetup,
    Vec<VerifierSetLeaf>,
    MerkleTree<SolanaSyscallHasher>,
    SecretKey,
    SecretKey,
) {
    let mollusk = Mollusk::new(
        &GATEWAY_PROGRAM_ID,
        "../../target/deploy/axelar_solana_gateway_v2",
    );

    let payer = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let operator = Pubkey::new_unique();

    // Step 1: Create REAL signers first
    let (secret_key_1, compressed_pubkey_1) = generate_random_signer();
    let (secret_key_2, compressed_pubkey_2) = generate_random_signer();

    let epoch = U256::from(1);
    let previous_verifier_retention = U256::from(5);
    let domain_separator = [2u8; 32];
    let minimum_rotation_delay = 3600;

    // Step 2: Create verifier set with your 2 real signers
    let quorum_threshold = 100;
    let verifier_leaves = vec![
        VerifierSetLeaf::new(
            0,
            quorum_threshold,
            PublicKey::Secp256k1(compressed_pubkey_1),
            50,
            0,
            2,
            domain_separator,
        ),
        VerifierSetLeaf::new(
            0,
            quorum_threshold,
            PublicKey::Secp256k1(compressed_pubkey_2),
            50,
            1,
            2,
            domain_separator,
        ),
    ];

    // Step 3: Calculate the REAL verifier set hash
    let verifier_leaf_hashes: Vec<[u8; 32]> =
        verifier_leaves.iter().map(|leaf| leaf.hash()).collect();
    let verifier_merkle_tree =
        MerkleTree::<SolanaSyscallHasher>::from_leaves(&verifier_leaf_hashes);
    let verifier_set_hash = verifier_merkle_tree.root().unwrap();

    // Step 4: Derive PDAs with the REAL verifier set hash
    let (gateway_root_pda, gateway_bump) =
        Pubkey::find_program_address(&[GATEWAY_SEED], &GATEWAY_PROGRAM_ID);

    let (program_data_pda, _) = Pubkey::find_program_address(
        &[GATEWAY_PROGRAM_ID.as_ref()],
        &solana_sdk::bpf_loader_upgradeable::id(),
    );

    let (verifier_set_tracker_pda, verifier_bump) = Pubkey::find_program_address(
        &[VERIFIER_SET_TRACKER_SEED, &verifier_set_hash],
        &GATEWAY_PROGRAM_ID,
    );

    let setup = TestSetup {
        mollusk,
        payer,
        upgrade_authority,
        operator,
        gateway_root_pda,
        gateway_bump,
        program_data_pda,
        verifier_set_tracker_pda,
        verifier_bump,
        verifier_set_hash,
        domain_separator,
        minimum_rotation_delay,
        epoch,
        previous_verifier_retention,
        gateway_caller_pda: None,
        gateway_caller_bump: None,
        event_authority_pda: None,
        event_authority_bump: None,
    };

    (
        setup,
        verifier_leaves,
        verifier_merkle_tree,
        secret_key_1,
        secret_key_2,
    )
}

pub fn initialize_gateway(setup: &TestSetup) -> InstructionResult {
    let params = InitializeConfigParams {
        domain_separator: setup.domain_separator,
        initial_verifier_set: InitialVerifierSet {
            hash: setup.verifier_set_hash,
            pda: setup.verifier_set_tracker_pda,
        },
        minimum_rotation_delay: setup.minimum_rotation_delay,
        operator: setup.operator,
        previous_verifier_retention: setup.previous_verifier_retention,
    };

    let instruction_data =
        axelar_solana_gateway_v2::instruction::InitializeConfig { params }.data();

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
            setup.gateway_root_pda,
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
        (
            setup.verifier_set_tracker_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
    ];

    let instruction = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(setup.upgrade_authority, true),
            AccountMeta::new_readonly(setup.program_data_pda, false),
            AccountMeta::new(setup.gateway_root_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(setup.verifier_set_tracker_pda, false),
        ],
        data: instruction_data,
    };

    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn initialize_payload_verification_session(
    setup: &TestSetup,
    init_result: &InstructionResult,
) -> (InstructionResult, Pubkey) {
    let initialized_gateway_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let initialized_verifier_set_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.verifier_set_tracker_pda)
        .unwrap()
        .1
        .clone();

    let merkle_root = [3u8; 32];
    let signing_verifier_set_hash = setup.verifier_set_hash;

    let (verification_session_pda, _verification_bump) = Pubkey::find_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            &merkle_root,
            &signing_verifier_set_hash,
        ],
        &GATEWAY_PROGRAM_ID,
    );

    let instruction_data =
        axelar_solana_gateway_v2::instruction::InitializePayloadVerificationSession { merkle_root }
            .data();

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
        (setup.gateway_root_pda, initialized_gateway_account),
        (
            verification_session_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.verifier_set_tracker_pda,
            initialized_verifier_set_account,
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
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(setup.gateway_root_pda, false),
            AccountMeta::new(verification_session_pda, false),
            AccountMeta::new_readonly(setup.verifier_set_tracker_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    (
        setup.mollusk.process_instruction(&instruction, &accounts),
        verification_session_pda,
    )
}

pub fn generate_random_signer() -> (SecretKey, [u8; 33]) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret_key_bytes: [u8; 32] = rng.gen();
    let secret_key = libsecp256k1::SecretKey::parse(&secret_key_bytes).unwrap();
    let public_key = libsecp256k1::PublicKey::from_secret_key(&secret_key);
    let compressed_pubkey = public_key.serialize_compressed();

    (secret_key, compressed_pubkey)
}

pub fn create_test_message(
    source_chain: &str,
    message_id: &str,
    destination_address: &str,
    payload_hash: [u8; 32],
) -> Message {
    Message {
        cc_id: CrossChainId {
            chain: source_chain.to_string(),
            id: message_id.to_string(),
        },
        source_address: "0xSourceAddress".to_string(),
        destination_chain: "solana".to_string(),
        destination_address: destination_address.to_string(),
        payload_hash,
    }
}

pub fn initialize_payload_verification_session_with_root(
    setup: &TestSetup,
    init_result: &InstructionResult,
    payload_merkle_root: [u8; 32],
) -> (InstructionResult, Pubkey) {
    let initialized_gateway_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let initialized_verifier_set_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.verifier_set_tracker_pda)
        .unwrap()
        .1
        .clone();

    let signing_verifier_set_hash = setup.verifier_set_hash;

    let (verification_session_pda, _) = Pubkey::find_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            &payload_merkle_root,
            &signing_verifier_set_hash,
        ],
        &GATEWAY_PROGRAM_ID,
    );

    let instruction_data =
        axelar_solana_gateway_v2::instruction::InitializePayloadVerificationSession {
            merkle_root: payload_merkle_root,
        }
        .data();

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
        (setup.gateway_root_pda, initialized_gateway_account),
        (
            verification_session_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            setup.verifier_set_tracker_pda,
            initialized_verifier_set_account,
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
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(setup.gateway_root_pda, false),
            AccountMeta::new(verification_session_pda, false),
            AccountMeta::new_readonly(setup.verifier_set_tracker_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    (
        setup.mollusk.process_instruction(&instruction, &accounts),
        verification_session_pda,
    )
}

pub fn create_verifier_info(
    secret_key: &SecretKey,
    payload_merkle_root: [u8; 32],
    verifier_leaf: &VerifierSetLeaf,
    position: usize,
    verifier_merkle_tree: &MerkleTree<SolanaSyscallHasher>,
) -> SigningVerifierSetInfo {
    let hashed_message =
        axelar_solana_gateway_v2::SignatureVerificationSessionData::prefixed_message_hash(
            &payload_merkle_root,
        );

    let message = libsecp256k1::Message::parse(&hashed_message);
    let (signature, recovery_id) = libsecp256k1::sign(&message, secret_key);
    let mut signature_bytes = signature.serialize().to_vec();
    signature_bytes.push(recovery_id.serialize() + 27);
    let signature_array: [u8; 65] = signature_bytes.try_into().unwrap();

    let merkle_proof = verifier_merkle_tree.proof(&[position]);
    let merkle_proof_bytes = merkle_proof.to_bytes();

    SigningVerifierSetInfo::new(signature_array, *verifier_leaf, merkle_proof_bytes)
}

pub fn call_contract_helper(
    setup: &TestSetup,
    init_result: InstructionResult,
    memo_program_id: Pubkey,
) -> InstructionResult {
    let signing_pda = setup.gateway_caller_pda.unwrap();
    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let mut accounts = vec![
        (
            memo_program_id,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
        (
            signing_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: memo_program_id,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            event_authority_pda,
            Account {
                lamports: 1,
                data: vec![],
                owner: GATEWAY_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            GATEWAY_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::bpf_loader_upgradeable::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ];

    let gateway_root = init_result.get_account(&setup.gateway_root_pda).unwrap();
    accounts.push((setup.gateway_root_pda, gateway_root.clone()));

    let destination_chain = "ethereum".to_string();
    let destination_contract_address = "0xdeadbeef".to_string();
    let payload = b"memo test".to_vec();

    let signing_pda_bump = setup.gateway_caller_bump.unwrap();

    let ix_data = axelar_solana_gateway_v2::instruction::CallContract {
        destination_chain,
        destination_contract_address,
        payload,
        signing_pda_bump,
    }
    .data();

    // Full account metas (must include event_authority + program)
    let ix = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: axelar_solana_gateway_v2::accounts::CallContract {
            caller: memo_program_id,
            signing_pda: Some(signing_pda),
            gateway_root_pda: setup.gateway_root_pda,
            event_authority: event_authority_pda,
            program: GATEWAY_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data,
    };

    setup.mollusk.process_instruction(&ix, &accounts)
}

pub fn verify_signature_helper(
    setup: &TestSetup,
    payload_merkle_root: [u8; 32],
    verifier_info: SigningVerifierSetInfo,
    verification_session_pda: Pubkey,
    gateway_account: Account,
    verification_session_account: Account,
    verifier_set_tracker_pda: Pubkey,
    verifier_set_tracker_account: Account,
) -> InstructionResult {
    let instruction_data = axelar_solana_gateway_v2::instruction::VerifySignature {
        payload_merkle_root,
        verifier_info,
    }
    .data();

    let accounts = vec![
        (setup.gateway_root_pda, gateway_account),
        (verification_session_pda, verification_session_account),
        (verifier_set_tracker_pda, verifier_set_tracker_account),
    ];

    let instruction = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(setup.gateway_root_pda, false),
            AccountMeta::new(verification_session_pda, false),
            AccountMeta::new_readonly(verifier_set_tracker_pda, false),
        ],
        data: instruction_data,
    };

    // Execute the verify_signature instruction
    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn rotate_signers_helper(
    setup: &TestSetup,
    new_verifier_set_hash: [u8; 32],
    verification_session_pda: Pubkey,
    verify_result: InstructionResult,
) -> InstructionResult {
    let instruction_data = axelar_solana_gateway_v2::instruction::RotateSigners {
        new_verifier_set_merkle_root: new_verifier_set_hash,
    }
    .data();

    let (new_verifier_set_tracker_pda, _) = Pubkey::find_program_address(
        &[VERIFIER_SET_TRACKER_SEED, new_verifier_set_hash.as_slice()],
        &GATEWAY_PROGRAM_ID,
    );

    let final_gateway_account = verify_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let final_verification_session_account = verify_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == verification_session_pda)
        .unwrap()
        .1
        .clone();

    let verifier_set_tracker_account = verify_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.verifier_set_tracker_pda)
        .unwrap()
        .1
        .clone();

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let accounts = vec![
        (setup.gateway_root_pda, final_gateway_account),
        (verification_session_pda, final_verification_session_account),
        (setup.verifier_set_tracker_pda, verifier_set_tracker_account),
        (
            new_verifier_set_tracker_pda,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
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
            SYSTEM_PROGRAM_ID,
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
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
        // for cpi events
        (
            event_authority_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
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
    ];

    let instruction = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.gateway_root_pda, false),
            AccountMeta::new_readonly(verification_session_pda, false),
            AccountMeta::new_readonly(setup.verifier_set_tracker_pda, false),
            AccountMeta::new(new_verifier_set_tracker_pda, false),
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            // optional operator
            AccountMeta::new(setup.operator, true),
            // for event cpi
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(GATEWAY_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn transfer_operatorship_helper(
    setup: &TestSetup,
    init_result: InstructionResult,
    new_operator: Pubkey,
) -> InstructionResult {
    let instruction_data = axelar_solana_gateway_v2::instruction::TransferOperatorship {}.data();

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let gateway_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let program_data_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.program_data_pda)
        .unwrap()
        .1
        .clone();

    let accounts = vec![
        (setup.gateway_root_pda, gateway_account),
        (setup.program_data_pda, program_data_account),
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
            new_operator,
            Account {
                lamports: 0,
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        ),
        // for cpi events
        (
            event_authority_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
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
    ];

    let instruction = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(setup.gateway_root_pda, false),
            AccountMeta::new_readonly(setup.operator, true),
            AccountMeta::new_readonly(setup.program_data_pda, false),
            AccountMeta::new_readonly(new_operator, false),
            // for CPI events
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(GATEWAY_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    // Execute the instruction
    setup.mollusk.process_instruction(&instruction, &accounts)
}

pub fn setup_message_merkle_tree(
    setup: &TestSetup,
    _verifier_set_merkle_root: [u8; 32],
) -> (
    Vec<Message>,
    Vec<MessageLeaf>,
    MerkleTree<SolanaSyscallHasher>,
    [u8; 32],
) {
    let messages = vec![
        create_test_message(
            "ethereum",
            "msg_1",
            "DNHKNbf4JWJNnquuWJuNUSFGsXbDYs1sPR1ZvVhah827",
            [1u8; 32],
        ),
        create_test_message(
            "ethereum",
            "msg_2",
            "8q49wyQjNrSEZf5A8h6jR7dwLnDxdnURftv89FWLWMGK",
            [2u8; 32],
        ),
    ];

    let message_leaves: Vec<MessageLeaf> = messages
        .iter()
        .enumerate()
        .map(|(i, msg)| MessageLeaf {
            message: msg.clone(),
            position: i as u16,
            set_size: messages.len() as u16,
            domain_separator: setup.domain_separator,
        })
        .collect();

    let message_leaf_hashes: Vec<[u8; 32]> =
        message_leaves.iter().map(|leaf| leaf.hash()).collect();

    let message_merkle_tree = MerkleTree::<SolanaSyscallHasher>::from_leaves(&message_leaf_hashes);

    let payload_merkle_root = message_merkle_tree.root().unwrap();

    (
        messages,
        message_leaves,
        message_merkle_tree,
        payload_merkle_root,
    )
}

pub fn approve_message_helper(
    setup: &TestSetup,
    message_merkle_tree: MerkleTree<SolanaSyscallHasher>,
    message_leaves: Vec<MessageLeaf>,
    messages: &[Message],
    payload_merkle_root: [u8; 32],
    verification_session_pda: Pubkey,
    verify_result_2: InstructionResult,
    position: usize,
) -> (InstructionResult, Pubkey) {
    let message_proof = message_merkle_tree.proof(&[position]);
    let message_proof_bytes = message_proof.to_bytes();

    let merkleised_message = MerkleisedMessage {
        leaf: message_leaves[position].clone(),
        proof: message_proof_bytes,
    };

    let cc_id = &messages[position].cc_id;
    let command_id =
        solana_program::keccak::hashv(&[cc_id.chain.as_bytes(), b"-", cc_id.id.as_bytes()]).0;

    let (incoming_message_pda, _incoming_message_bump) = Pubkey::find_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, &command_id],
        &GATEWAY_PROGRAM_ID,
    );

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &GATEWAY_PROGRAM_ID);

    let approve_instruction_data = axelar_solana_gateway_v2::instruction::ApproveMessage {
        merkleised_message: merkleised_message.clone(),
        payload_merkle_root,
    }
    .data();

    let final_gateway_account = verify_result_2
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let final_verification_session_account = verify_result_2
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == verification_session_pda)
        .unwrap()
        .1
        .clone();

    let approve_accounts = vec![
        (setup.gateway_root_pda, final_gateway_account),
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
        (verification_session_pda, final_verification_session_account),
        (
            incoming_message_pda,
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
        (
            event_authority_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: vec![],
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
    ];

    let approve_instruction = Instruction {
        program_id: GATEWAY_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(setup.gateway_root_pda, false),
            AccountMeta::new(setup.payer, true),
            AccountMeta::new_readonly(verification_session_pda, false),
            AccountMeta::new(incoming_message_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(GATEWAY_PROGRAM_ID, false),
        ],
        data: approve_instruction_data,
    };

    (
        setup
            .mollusk
            .process_instruction(&approve_instruction, &approve_accounts),
        incoming_message_pda,
    )
}

pub fn approve_messages_on_gateway(
    setup: &TestSetup,
    messages: Vec<Message>,
    init_result: InstructionResult,
    secret_key_1: &SecretKey,
    secret_key_2: &SecretKey,
    verifier_leaves: Vec<VerifierSetLeaf>,
    verifier_merkle_tree: MerkleTree<SolanaSyscallHasher>,
) -> Vec<(IncomingMessage, Pubkey, Vec<u8>)> {
    let (messages, message_leaves, message_merkle_tree, payload_merkle_root) =
        setup_message_merkle_tree_from_messages(setup, messages);

    let (session_result, verification_session_pda) =
        initialize_payload_verification_session_with_root(setup, &init_result, payload_merkle_root);
    assert!(!session_result.program_result.is_err());

    let gateway_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.gateway_root_pda)
        .unwrap()
        .1
        .clone();

    let verifier_set_tracker_account = init_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.verifier_set_tracker_pda)
        .unwrap()
        .1
        .clone();

    let verification_session_account = session_result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == verification_session_pda)
        .unwrap()
        .1
        .clone();

    let verifier_info_1 = create_verifier_info(
        secret_key_1,
        payload_merkle_root,
        &verifier_leaves[0],
        0,
        &verifier_merkle_tree,
    );

    let verify_result_1 = verify_signature_helper(
        setup,
        payload_merkle_root,
        verifier_info_1,
        verification_session_pda,
        gateway_account.clone(),
        verification_session_account.clone(),
        setup.verifier_set_tracker_pda,
        verifier_set_tracker_account.clone(),
    );

    let updated_verification_account_after_first = verify_result_1
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == verification_session_pda)
        .unwrap()
        .1
        .clone();

    let verifier_info_2 = create_verifier_info(
        secret_key_2,
        payload_merkle_root,
        &verifier_leaves[1],
        1,
        &verifier_merkle_tree,
    );

    let verify_result_2 = verify_signature_helper(
        setup,
        payload_merkle_root,
        verifier_info_2,
        verification_session_pda,
        gateway_account,
        updated_verification_account_after_first,
        setup.verifier_set_tracker_pda,
        verifier_set_tracker_account,
    );

    let mut incoming_messages = Vec::new();

    // Approve all messages
    for i in 0..messages.len() {
        // Step 8: Approve the message
        let (approve_result, incoming_message_pda) = approve_message_helper(
            setup,
            message_merkle_tree.clone(),
            message_leaves.clone(),
            &messages,
            payload_merkle_root,
            verification_session_pda,
            verify_result_2.clone(),
            i, // message position
        );

        let incoming_message_account = approve_result
            .resulting_accounts
            .iter()
            .find(|(pubkey, _)| *pubkey == incoming_message_pda)
            .unwrap()
            .1
            .clone();

        // sanity check
        let incoming_message =
            IncomingMessage::try_deserialize(&mut incoming_message_account.data.as_slice())
                .unwrap();

        incoming_messages.push((
            incoming_message,
            incoming_message_pda,
            incoming_message_account.data,
        ));
    }

    incoming_messages
}

pub fn setup_message_merkle_tree_from_messages(
    setup: &TestSetup,
    messages: Vec<Message>,
) -> (
    Vec<Message>,
    Vec<MessageLeaf>,
    MerkleTree<SolanaSyscallHasher>,
    [u8; 32],
) {
    let message_leaves: Vec<MessageLeaf> = messages
        .iter()
        .enumerate()
        .map(|(i, msg)| MessageLeaf {
            message: msg.clone(),
            position: i as u16,
            set_size: messages.len() as u16,
            domain_separator: setup.domain_separator,
        })
        .collect();

    let message_leaf_hashes: Vec<[u8; 32]> =
        message_leaves.iter().map(|leaf| leaf.hash()).collect();

    let message_merkle_tree = MerkleTree::<SolanaSyscallHasher>::from_leaves(&message_leaf_hashes);

    let payload_merkle_root = message_merkle_tree.root().unwrap();

    (
        messages,
        message_leaves,
        message_merkle_tree,
        payload_merkle_root,
    )
}

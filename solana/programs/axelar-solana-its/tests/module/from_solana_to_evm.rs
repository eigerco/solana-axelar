#![allow(clippy::too_many_lines)]
#![allow(clippy::shadow_unrelated)]
#![allow(clippy::panic)]

use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use axelar_solana_its::instructions::{
    DeployInterchainTokenInputs, DeployTokenManagerInputs, InterchainTransferInputs,
};
use axelar_solana_its::state::token_manager;
use evm_contracts_test_suite::ethers::abi::Detokenize;
use evm_contracts_test_suite::ethers::contract::{ContractCall, EthLogDecode, Event as EvmEvent};
use evm_contracts_test_suite::ethers::providers::Middleware;
use evm_contracts_test_suite::ethers::signers::Signer;
use evm_contracts_test_suite::ethers::types::{Address, TransactionReceipt, U256};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::{
    AxelarAmplifierGateway as EvmAxelarAmplifierGateway, Message as EvmAxelarMessage,
    Proof as EvmAxelarProof,
};
use evm_contracts_test_suite::{evm_weighted_signers, ContractMiddleware};
use gateway::events::{ArchivedCallContract, ArchivedGatewayEvent, EventContainer, GatewayEvent};
use rstest::rstest;
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer as SolanaSigner};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::instruction::AuthorityType;
use spl_token_2022::state::Mint;
use test_fixtures::test_setup::TestFixture;

use crate::{axelar_evm_setup, axelar_solana_setup, ItsProgramWrapper};

#[tokio::test]
async fn test_send_deploy_interchain_token_from_solana_to_evm() {
    // InterchainTokens deployed through ITS are always spl-token-2022 programs,
    // hence we only test spl-token-2022 here.

    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .salt(salt)
        .minter(evm_signer.wallet.address().as_bytes().to_owned())
        .destination_chain(destination_chain)
        .gas_value(0_u128)
        .build();

    let ix = axelar_solana_its::instructions::deploy_interchain_token(deploy).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_token_deployed_filter(),
    )
    .await;
    let expected_token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        solana_sdk::keccak::hash(b"our cool interchain token")
            .0
            .as_slice(),
    );

    assert_eq!(log.token_id, expected_token_id, "token_id does not match");
}

#[tokio::test]
async fn test_send_deploy_token_manager_from_solana_to_evm() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let token_name = "TestToken";
    let token_symbol = "TT";
    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let token_address =
        alloy_primitives::Address::from(test_its_canonical_token.address().to_fixed_bytes());
    let params = (Bytes::new(), token_address).abi_encode_params();

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain)
        .token_manager_type(token_manager::Type::LockUnlock)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;
    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(
        alloy_primitives::U256::from(log.token_manager_type),
        token_manager::Type::LockUnlock.into(),
    );
}

#[rstest]
#[tokio::test]
async fn test_send_interchain_transfer_from_solana_to_evm_native() {
    // InterchainTokens deployed through ITS are always spl-token-2022 programs,
    // hence we only test spl-token-2022 here.

    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy_local = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .salt(salt)
        .minter(solana_chain.fixture.payer.pubkey().as_ref().to_vec())
        .gas_value(0_u128)
        .build();

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_interchain_token(deploy_local).unwrap();
    solana_chain.fixture.send_tx(&[deploy_local_ix]).await;

    let deploy_remote = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .destination_chain(destination_chain.clone())
        .salt(salt)
        .minter(evm_signer.wallet.address().as_bytes().to_vec())
        .gas_value(0_u128)
        .build();
    let deploy_remote_ix =
        axelar_solana_its::instructions::deploy_interchain_token(deploy_remote).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, deploy_remote_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );

    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_token_deployed_filter(),
    )
    .await;

    let expected_token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        solana_sdk::keccak::hash(b"our cool interchain token")
            .0
            .as_slice(),
    );

    assert_eq!(log.token_id, expected_token_id, "token_id does not match");

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &log.token_id);

    let associated_account_address = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let mint_ix = axelar_solana_its::instructions::mint_to(
        expected_token_id,
        interchain_token_pda,
        associated_account_address,
        solana_chain.fixture.payer.pubkey(),
        spl_token_2022::id(),
        500,
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[mint_ix]).await;

    let transfer = InterchainTransferInputs::builder()
        .authority(solana_chain.fixture.payer.pubkey())
        .source_account(associated_account_address)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(323)
        .gas_value(0_u128)
        .metadata(vec![])
        .build();

    let transfer_ix = axelar_solana_its::instructions::interchain_transfer(transfer).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, transfer_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    assert_eq!(log.amount, U256::from(323_u64));
}

#[rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
async fn test_send_interchain_transfer_from_solana_to_evm_mint_burn(
    #[case] token_program_id: Pubkey,
) {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let token_name = "TestToken";
    let token_symbol = "TT";
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);

    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;
    let params = axelar_solana_its::state::token_manager::encode_params(None, mint);
    let deploy_local = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .token_manager_type(token_manager::Type::MintBurn)
        .gas_value(0)
        .params(params)
        .token_program(token_program_id)
        .build();

    let transfer_mint_authority_ix = spl_token_2022::instruction::set_authority(
        &token_program_id,
        &mint,
        Some(&token_manager_pda),
        AuthorityType::MintTokens,
        &solana_chain.fixture.payer.pubkey(),
        &[],
    )
    .unwrap();

    solana_chain
        .fixture
        .send_tx(&[transfer_mint_authority_ix])
        .await;

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_token_manager(deploy_local).unwrap();
    solana_chain.fixture.send_tx(&[deploy_local_ix]).await;

    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let token_address =
        alloy_primitives::Address::from(test_its_canonical_token.address().to_fixed_bytes());
    let params = (Bytes::new(), token_address).abi_encode_params();
    let deploy_remote_ix = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain.clone())
        .token_manager_type(token_manager::Type::MintBurn)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy_remote_ix).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;
    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(
        alloy_primitives::U256::from(log.token_manager_type),
        token_manager::Type::MintBurn.into(),
    );

    let evm_token_manager_address = its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .await
        .unwrap();

    call_evm(test_its_canonical_token.add_minter(evm_token_manager_address)).await;

    let ata = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;
    let initial_amount = 3000;
    let transfer_amount = 333;

    let mint_tokens_ix = axelar_solana_its::instructions::mint_to(
        token_id,
        mint,
        ata,
        solana_chain.fixture.payer.pubkey(),
        token_program_id,
        initial_amount,
    )
    .unwrap();
    solana_chain.fixture.send_tx(&[mint_tokens_ix]).await;

    let transfer = InterchainTransferInputs::builder()
        .mint(mint)
        .authority(solana_chain.fixture.payer.pubkey())
        .source_account(ata)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(transfer_amount)
        .gas_value(0_u128)
        .token_program(token_program_id)
        .metadata(vec![])
        .build();

    let transfer_ix = axelar_solana_its::instructions::interchain_transfer(transfer).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, transfer_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    let ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(ata)
        .await
        .unwrap();

    assert_eq!(ata.amount, initial_amount - transfer_amount);
    assert_eq!(log.amount, U256::from(transfer_amount));
}
#[rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
async fn test_send_interchain_transfer_from_solana_to_evm_mint_burn_from(
    #[case] token_program_id: Pubkey,
) {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let token_name = "TestToken";
    let token_symbol = "TT";
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);

    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;
    let params = axelar_solana_its::state::token_manager::encode_params(None, mint);
    let deploy_local = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .token_manager_type(token_manager::Type::MintBurnFrom)
        .gas_value(0)
        .params(params)
        .token_program(token_program_id)
        .build();

    let transfer_mint_authority_ix = spl_token_2022::instruction::set_authority(
        &token_program_id,
        &mint,
        Some(&token_manager_pda),
        AuthorityType::MintTokens,
        &solana_chain.fixture.payer.pubkey(),
        &[],
    )
    .unwrap();

    solana_chain
        .fixture
        .send_tx(&[transfer_mint_authority_ix])
        .await;

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_token_manager(deploy_local).unwrap();
    solana_chain.fixture.send_tx(&[deploy_local_ix]).await;

    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let token_address =
        alloy_primitives::Address::from(test_its_canonical_token.address().to_fixed_bytes());
    let params = (Bytes::new(), token_address).abi_encode_params();
    let deploy_remote_ix = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain.clone())
        .token_manager_type(token_manager::Type::MintBurnFrom)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy_remote_ix).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;
    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(
        alloy_primitives::U256::from(log.token_manager_type),
        token_manager::Type::MintBurnFrom.into(),
    );

    let evm_token_manager_address = its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .await
        .unwrap();

    call_evm(test_its_canonical_token.add_minter(evm_token_manager_address)).await;

    let bob = Keypair::new();

    let bob_ata =
        get_associated_token_address_with_program_id(&bob.pubkey(), &mint, &token_program_id);

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &bob.pubkey(),
        &mint,
        &token_program_id,
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;
    let initial_amount = 3000;
    let delegated_amount = 800;
    let transfer_amount = 333;

    let mint_tokens_ix = axelar_solana_its::instructions::mint_to(
        token_id,
        mint,
        bob_ata,
        solana_chain.fixture.payer.pubkey(),
        token_program_id,
        initial_amount,
    )
    .unwrap();
    solana_chain.fixture.send_tx(&[mint_tokens_ix]).await;

    let approve_ix = spl_token_2022::instruction::approve(
        &token_program_id,
        &bob_ata,
        &token_manager_pda,
        &bob.pubkey(),
        &[],
        delegated_amount,
    )
    .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[approve_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let transfer = InterchainTransferInputs::builder()
        .mint(mint)
        .source_account(bob_ata)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(transfer_amount)
        .gas_value(0_u128)
        .token_program(token_program_id)
        .metadata(vec![])
        .build();

    let transfer_ix = axelar_solana_its::instructions::interchain_transfer(transfer).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, transfer_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    let bob_ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(bob_ata)
        .await
        .unwrap();

    assert_eq!(bob_ata.amount, initial_amount - transfer_amount);
    assert_eq!(bob_ata.delegated_amount, delegated_amount - transfer_amount);
    assert_eq!(log.amount, U256::from(transfer_amount));
}

#[rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
async fn test_send_interchain_transfer_from_solana_to_evm_lock_unlock(
    #[case] token_program_id: Pubkey,
) {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let token_name = "TestToken";
    let token_symbol = "TT";
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);

    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;

    let params = axelar_solana_its::state::token_manager::encode_params(None, mint);
    let deploy_local = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .token_manager_type(token_manager::Type::LockUnlock)
        .gas_value(0)
        .params(params)
        .token_program(token_program_id)
        .build();

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_token_manager(deploy_local).unwrap();
    solana_chain.fixture.send_tx(&[deploy_local_ix]).await;

    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let token_address =
        alloy_primitives::Address::from(test_its_canonical_token.address().to_fixed_bytes());
    let params = (Bytes::new(), token_address).abi_encode_params();
    let deploy_remote_ix = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain.clone())
        .token_manager_type(token_manager::Type::MintBurn)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy_remote_ix).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;
    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(
        alloy_primitives::U256::from(log.token_manager_type),
        token_manager::Type::MintBurn.into(),
    );

    let evm_token_manager_address = its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .await
        .unwrap();

    call_evm(test_its_canonical_token.add_minter(evm_token_manager_address)).await;

    let ata = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let initial_amount = 3000;
    let transfer_amount = 333;

    solana_chain
        .fixture
        .mint_tokens_to(
            mint,
            ata,
            solana_chain.fixture.payer.insecure_clone(),
            initial_amount,
            token_program_id,
        )
        .await;

    let transfer = InterchainTransferInputs::builder()
        .authority(solana_chain.fixture.payer.pubkey())
        .mint(mint)
        .source_account(ata)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(transfer_amount)
        .gas_value(0_u128)
        .token_program(token_program_id)
        .metadata(vec![])
        .build();

    let transfer_ix = axelar_solana_its::instructions::interchain_transfer(transfer).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, transfer_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    let ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(ata)
        .await
        .unwrap();

    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program_id);
    let token_manager_ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(token_manager_ata)
        .await
        .unwrap();

    assert_eq!(ata.amount, initial_amount - transfer_amount);
    assert_eq!(log.amount, U256::from(transfer_amount));
    assert_eq!(token_manager_ata.amount, transfer_amount);
}

#[tokio::test]
async fn test_send_interchain_transfer_from_solana_to_evm_lock_unlock_fee() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let token_name = "TestToken";
    let token_symbol = "TT";
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);

    let fee_basis_points = 50_u16;
    let maximum_fee = u64::MAX;
    let mint = solana_chain
        .fixture
        .init_new_mint_with_fee(
            solana_chain.fixture.payer.pubkey(),
            spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            18,
            None,
            None,
        )
        .await;

    let params = axelar_solana_its::state::token_manager::encode_params(None, mint);
    let deploy_local = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .token_manager_type(token_manager::Type::LockUnlockFee)
        .gas_value(0)
        .params(params)
        .token_program(spl_token_2022::id())
        .build();

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_token_manager(deploy_local).unwrap();
    solana_chain.fixture.send_tx(&[deploy_local_ix]).await;

    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let token_address =
        alloy_primitives::Address::from(test_its_canonical_token.address().to_fixed_bytes());
    let params = (Bytes::new(), token_address).abi_encode_params();
    let deploy_remote_ix = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain.clone())
        .token_manager_type(token_manager::Type::MintBurn)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy_remote_ix).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    call_evm(
        its_contracts
            .interchain_token_service
            .set_trusted_address(message.source_chain.clone(), message.source_address.clone()),
    )
    .await;
    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(
        alloy_primitives::U256::from(log.token_manager_type),
        token_manager::Type::MintBurn.into(),
    );

    let evm_token_manager_address = its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .await
        .unwrap();

    call_evm(test_its_canonical_token.add_minter(evm_token_manager_address)).await;

    let ata = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &spl_token_2022::id(),
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let initial_amount = 3000;
    let transfer_amount = 333;

    solana_chain
        .fixture
        .mint_tokens_to(
            mint,
            ata,
            solana_chain.fixture.payer.insecure_clone(),
            initial_amount,
            spl_token_2022::id(),
        )
        .await;

    let transfer = InterchainTransferInputs::builder()
        .authority(solana_chain.fixture.payer.pubkey())
        .mint(mint)
        .source_account(ata)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(transfer_amount)
        .gas_value(0_u128)
        .metadata(vec![])
        .build();

    let transfer_ix = axelar_solana_its::instructions::interchain_transfer(transfer).unwrap();
    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, transfer_ix).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    let command_id = ensure_evm_gateway_approval(messages, proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        call_contract.payload.to_vec().into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    let ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(ata)
        .await
        .unwrap();

    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint,
        &spl_token_2022::id(),
    );
    let token_manager_ata = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(token_manager_ata)
        .await
        .unwrap();

    assert_eq!(ata.amount, initial_amount - transfer_amount);

    let mint_data = solana_chain
        .fixture
        .banks_client
        .get_account(mint)
        .await
        .unwrap()
        .unwrap();

    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let epoch = solana_chain
        .fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .epoch;
    let fee = fee_config
        .calculate_epoch_fee(epoch, transfer_amount)
        .unwrap();

    assert_eq!(
        log.amount,
        U256::from(transfer_amount.checked_sub(fee).unwrap())
    );
    assert_eq!(
        token_manager_ata.amount,
        transfer_amount.checked_sub(fee).unwrap()
    );
}

async fn retrieve_evm_log_with_filter<M, T>(filter: EvmEvent<std::sync::Arc<M>, M, T>) -> T
where
    M: Middleware,
    T: EthLogDecode,
{
    filter
        .from_block(0_u64)
        .query()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("no logs found")
}

async fn call_evm<M, D>(contract_call: ContractCall<M, D>) -> TransactionReceipt
where
    M: Middleware,
    D: Detokenize,
{
    contract_call.send().await.unwrap().await.unwrap().unwrap()
}

async fn ensure_evm_gateway_approval(
    messages: Vec<EvmAxelarMessage>,
    proof: EvmAxelarProof,
    gateway: &EvmAxelarAmplifierGateway<ContractMiddleware>,
) -> [u8; 32] {
    let message = messages[0].clone();

    call_evm(gateway.approve_messages(messages, proof)).await;

    let is_approved = gateway
        .is_message_approved(
            message.source_chain.clone(),
            message.message_id.clone(),
            message.source_address.clone(),
            message.contract_address,
            message.payload_hash,
        )
        .await
        .unwrap();

    assert!(is_approved, "contract call was not approved");

    gateway
        .message_to_command_id(message.source_chain.clone(), message.message_id.clone())
        .await
        .unwrap()
}

fn prepare_evm_approve_contract_call(
    solana_id: &str,
    call_contract: &ArchivedCallContract,
    destination_address: Address,
    signer_set: &mut evm_weighted_signers::WeightedSigners,
    domain_separator: [u8; 32],
) -> (Vec<EvmAxelarMessage>, EvmAxelarProof) {
    // TODO: use address from the contract call once we have the trusted addresses
    // in place (the address is currently empty)
    let message = EvmAxelarMessage {
        source_chain: solana_id.to_string(),
        message_id: String::from_utf8_lossy(&call_contract.payload_hash).to_string(),
        source_address: Pubkey::from(call_contract.sender).to_string(),
        contract_address: destination_address,
        payload_hash: call_contract.payload_hash,
    };

    let approve_contract_call_command =
        evm_weighted_signers::get_approve_contract_call(message.clone());

    // Build command batch
    let signed_weighted_execute_input = evm_weighted_signers::get_weighted_signatures_proof(
        &approve_contract_call_command,
        signer_set,
        domain_separator,
    );

    (vec![message], signed_weighted_execute_input)
}

async fn call_solana_gateway(solana_fixture: &mut TestFixture, ix: Instruction) -> EventContainer {
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&solana_fixture.payer.pubkey()),
        &[&solana_fixture.payer],
        solana_fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap(),
    );
    let tx = solana_fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");

    let log_msgs = tx.metadata.unwrap().log_messages;
    let gateway_event = log_msgs
        .iter()
        .find_map(GatewayEvent::parse_log)
        .expect("Gateway event was not emitted?");

    gateway_event
}
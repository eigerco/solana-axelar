use alloy_primitives::Bytes;
use anyhow::anyhow;
use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::SendToHub;
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::program_pack::Pack as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use test_context::test_context;

use axelar_solana_gateway::processor::GatewayEvent;
use axelar_solana_gateway_test_fixtures::gateway::get_gateway_events;
use axelar_solana_gateway_test_fixtures::gateway::ProgramInvocationState;
use evm_contracts_test_suite::ethers::signers::Signer as EvmSigner;
use evm_contracts_test_suite::ethers::types::U256;
use interchain_token_transfer_gmp::GMPPayload;
use interchain_token_transfer_gmp::InterchainTransfer;

use crate::{retrieve_evm_log_with_filter, ItsTestContext};

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_incoming_interchain_transfer_within_limit(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &ctx.deployed_interchain_token);
    let token_program_id = spl_token_2022::id();
    let flow_limit = 800;

    let flow_limit_ix = axelar_solana_its::instruction::set_flow_limit(
        ctx.solana_wallet,
        ctx.deployed_interchain_token,
        flow_limit,
    )?;

    ctx.send_solana_tx(&[flow_limit_ix]).await;

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix]).await;

    let inner_transfer_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID.try_into()?,
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(InterchainTransfer {
            selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into()?,
            token_id: ctx.deployed_interchain_token.into(),
            source_address: [5; 32].into(),
            destination_address: associated_account_address.to_bytes().into(),
            amount: flow_limit.try_into()?,
            data: Bytes::new(),
        })
        .encode()
        .into(),
    })
    .encode();

    ctx.relay_to_solana(
        &inner_transfer_payload,
        Some(interchain_token_pda),
        token_program_id,
    )
    .await;

    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &ctx.solana_wallet,
            &interchain_token_pda,
            &token_program_id,
        );

    let destination_raw_account = ctx
        .solana_chain
        .try_get_account_no_checks(&destination_ata)
        .await?
        .ok_or_else(|| anyhow!("destination account not found"))?;
    let destination_ata_account =
        spl_token_2022::state::Account::unpack_from_slice(&destination_raw_account.data).unwrap();

    assert_eq!(
        destination_ata_account.amount, flow_limit,
        "New balance doesn't match expected balance"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[should_panic]
#[tokio::test]
async fn test_incoming_interchain_transfer_beyond_limit(ctx: &mut ItsTestContext) {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &ctx.deployed_interchain_token);
    let token_program_id = spl_token_2022::id();
    let flow_limit = 800;

    let flow_limit_ix = axelar_solana_its::instruction::set_flow_limit(
        ctx.solana_wallet,
        ctx.deployed_interchain_token,
        flow_limit,
    )
    .unwrap();

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix, flow_limit_ix])
        .await;

    let inner_transfer_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(InterchainTransfer {
            selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into().unwrap(),
            token_id: ctx.deployed_interchain_token.into(),
            source_address: [5; 32].into(),
            destination_address: associated_account_address.to_bytes().into(),
            amount: (flow_limit + 1).try_into().unwrap(),
            data: Bytes::new(),
        })
        .encode()
        .into(),
    })
    .encode();

    ctx.relay_to_solana(
        &inner_transfer_payload,
        Some(interchain_token_pda),
        token_program_id,
    )
    .await;

    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &ctx.solana_wallet,
            &interchain_token_pda,
            &token_program_id,
        );

    let destination_raw_account = ctx
        .solana_chain
        .try_get_account_no_checks(&destination_ata)
        .await
        .unwrap()
        .unwrap();
    let destination_ata_account =
        spl_token_2022::state::Account::unpack_from_slice(&destination_raw_account.data).unwrap();

    assert_eq!(
        destination_ata_account.amount, flow_limit,
        "New balance doesn't match expected balance"
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_outgoing_interchain_transfer_within_limit(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let token_id = ctx.deployed_interchain_token;
    let flow_limit = 800;

    let flow_limit_ix =
        axelar_solana_its::instruction::set_flow_limit(ctx.solana_wallet, token_id, flow_limit)?;

    ctx.send_solana_tx(&[flow_limit_ix]).await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix]).await;

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        interchain_token_pda,
        associated_account_address,
        ctx.solana_wallet,
        spl_token_2022::id(),
        900,
    )?;

    ctx.send_solana_tx(&[mint_ix]).await;

    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;

    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        associated_account_address,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        flow_limit,
        interchain_token_pda,
        spl_token_2022::id(),
        0,
        clock_sysvar.unix_timestamp,
    )?;

    let tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();
    let emitted_events = get_gateway_events(&tx)
        .pop()
        .ok_or_else(|| anyhow!("no events"))?;

    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };

    let [(_, GatewayEvent::CallContract(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };

    ctx.relay_to_evm(&emitted_event.payload).await;

    let log = retrieve_evm_log_with_filter(
        ctx.evm_its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    assert_eq!(log.amount, U256::from(800_u32));

    Ok(())
}

#[test_context(ItsTestContext)]
#[should_panic]
#[tokio::test]
async fn test_outgoing_interchain_transfer_outside_limit(ctx: &mut ItsTestContext) {
    let token_id = ctx.deployed_interchain_token;
    let flow_limit = 800;
    let flow_limit_ix =
        axelar_solana_its::instruction::set_flow_limit(ctx.solana_wallet, token_id, flow_limit)
            .unwrap();

    ctx.send_solana_tx(&[flow_limit_ix]).await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix]).await;

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        interchain_token_pda,
        associated_account_address,
        ctx.solana_wallet,
        spl_token_2022::id(),
        900,
    )
    .unwrap();

    ctx.send_solana_tx(&[mint_ix]).await;

    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;

    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        associated_account_address,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        flow_limit + 1,
        interchain_token_pda,
        spl_token_2022::id(),
        0,
        clock_sysvar.unix_timestamp,
    )
    .unwrap();

    ctx.send_solana_tx(&[transfer_ix]).await.unwrap();
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_flow_slot_initialization_incoming_transfer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &ctx.deployed_interchain_token);
    let token_program_id = spl_token_2022::id();
    let flow_limit = 500;
    let transfer_amount = 300;

    // Set flow limit
    let flow_limit_ix = axelar_solana_its::instruction::set_flow_limit(
        ctx.solana_wallet,
        ctx.deployed_interchain_token,
        flow_limit,
    )?;

    ctx.send_solana_tx(&[flow_limit_ix]).await;

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix]).await;

    // First incoming transfer - this should create a new flow slot with flow_in=transfer_amount
    let inner_transfer_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID.try_into()?,
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(InterchainTransfer {
            selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into()?,
            token_id: ctx.deployed_interchain_token.into(),
            source_address: [5; 32].into(),
            destination_address: associated_account_address.to_bytes().into(),
            amount: transfer_amount.try_into()?,
            data: Bytes::new(),
        })
        .encode()
        .into(),
    })
    .encode();

    ctx.relay_to_solana(
        &inner_transfer_payload,
        Some(interchain_token_pda),
        token_program_id,
    )
    .await;

    // Verify the token was minted
    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &ctx.solana_wallet,
            &interchain_token_pda,
            &token_program_id,
        );

    let destination_raw_account = ctx
        .solana_chain
        .try_get_account_no_checks(&destination_ata)
        .await?
        .ok_or_else(|| anyhow!("destination account not found"))?;
    let destination_ata_account =
        spl_token_2022::state::Account::unpack_from_slice(&destination_raw_account.data).unwrap();

    assert_eq!(
        destination_ata_account.amount, transfer_amount,
        "First transfer balance doesn't match expected"
    );

    // Second incoming transfer to ensure flow slot tracks correctly
    let second_transfer_amount = 100;
    let inner_transfer_payload_2 = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID.try_into()?,
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(InterchainTransfer {
            selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into()?,
            token_id: ctx.deployed_interchain_token.into(),
            source_address: [5; 32].into(),
            destination_address: associated_account_address.to_bytes().into(),
            amount: second_transfer_amount.try_into()?,
            data: Bytes::new(),
        })
        .encode()
        .into(),
    })
    .encode();

    ctx.relay_to_solana(
        &inner_transfer_payload_2,
        Some(interchain_token_pda),
        token_program_id,
    )
    .await;

    let destination_raw_account_2 = ctx
        .solana_chain
        .try_get_account_no_checks(&destination_ata)
        .await?
        .ok_or_else(|| anyhow!("destination account not found"))?;
    let destination_ata_account_2 =
        spl_token_2022::state::Account::unpack_from_slice(&destination_raw_account_2.data).unwrap();

    assert_eq!(
        destination_ata_account_2.amount,
        transfer_amount + second_transfer_amount,
        "Total balance doesn't match expected after two transfers"
    );

    // Check FlowSlot values on-chain
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &ctx.deployed_interchain_token);
    let current_timestamp = ctx.solana_chain.get_sysvar::<Clock>().await.unix_timestamp;
    let current_epoch =
        axelar_solana_its::state::flow_limit::flow_epoch_with_timestamp(current_timestamp)?;
    let (flow_slot_pda, _) =
        axelar_solana_its::find_flow_slot_pda(&token_manager_pda, current_epoch);

    let flow_slot_account = ctx
        .solana_chain
        .try_get_account_no_checks(&flow_slot_pda)
        .await?
        .ok_or_else(|| anyhow!("flow slot account not found"))?;

    let flow_slot =
        axelar_solana_its::state::flow_limit::FlowSlot::try_from_slice(&flow_slot_account.data)?;

    // For incoming transfers, flow_in should be set to the total transfer amount
    assert_eq!(
        flow_slot.flow_in,
        transfer_amount + second_transfer_amount,
        "flow_in doesn't match expected value"
    );
    assert_eq!(
        flow_slot.flow_out, 0,
        "flow_out should be 0 for incoming transfers"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_flow_slot_initialization_outgoing_transfer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let token_id = ctx.deployed_interchain_token;
    let flow_limit = 500;
    let transfer_amount = 300;

    // Set flow limit
    let flow_limit_ix =
        axelar_solana_its::instruction::set_flow_limit(ctx.solana_wallet, token_id, flow_limit)?;

    ctx.send_solana_tx(&[flow_limit_ix]).await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let associated_account_address = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    ctx.send_solana_tx(&[create_token_account_ix]).await;

    // Mint tokens to transfer
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        interchain_token_pda,
        associated_account_address,
        ctx.solana_wallet,
        spl_token_2022::id(),
        flow_limit + 100, // Mint more than flow limit to test multiple transfers
    )?;

    ctx.send_solana_tx(&[mint_ix]).await;

    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;

    // First outgoing transfer - this should create a new flow slot with flow_out=transfer_amount
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        associated_account_address,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        interchain_token_pda,
        spl_token_2022::id(),
        0,
        clock_sysvar.unix_timestamp,
    )?;

    let tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();
    let emitted_events = get_gateway_events(&tx)
        .pop()
        .ok_or_else(|| anyhow!("no events"))?;

    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };

    let [(_, GatewayEvent::CallContract(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };

    ctx.relay_to_evm(&emitted_event.payload).await;

    let log = retrieve_evm_log_with_filter(
        ctx.evm_its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    assert_eq!(log.amount, U256::from(transfer_amount));

    // Second outgoing transfer to ensure flow slot tracks correctly
    let second_transfer_amount = 100;
    let transfer_ix_2 = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        associated_account_address,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        second_transfer_amount,
        interchain_token_pda,
        spl_token_2022::id(),
        0,
        clock_sysvar.unix_timestamp,
    )?;

    let tx_2 = ctx.send_solana_tx(&[transfer_ix_2]).await.unwrap();
    let emitted_events_2 = get_gateway_events(&tx_2)
        .pop()
        .ok_or_else(|| anyhow!("no events"))?;

    let ProgramInvocationState::Succeeded(vec_events_2) = emitted_events_2 else {
        panic!("unexpected event")
    };

    let [(_, GatewayEvent::CallContract(emitted_event_2))] = vec_events_2.as_slice() else {
        panic!("unexpected event")
    };

    ctx.relay_to_evm(&emitted_event_2.payload).await;

    // The flow slot should have tracked both outgoing transfers correctly
    // Total flow_out should be transfer_amount + second_transfer_amount = 400

    // Check FlowSlot values on-chain
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let current_timestamp = ctx.solana_chain.get_sysvar::<Clock>().await.unix_timestamp;
    let current_epoch =
        axelar_solana_its::state::flow_limit::flow_epoch_with_timestamp(current_timestamp)?;
    let (flow_slot_pda, _) =
        axelar_solana_its::find_flow_slot_pda(&token_manager_pda, current_epoch);

    let flow_slot_account = ctx
        .solana_chain
        .try_get_account_no_checks(&flow_slot_pda)
        .await?
        .ok_or_else(|| anyhow!("flow slot account not found"))?;

    let flow_slot =
        axelar_solana_its::state::flow_limit::FlowSlot::try_from_slice(&flow_slot_account.data)?;

    // For outgoing transfers, flow_out should be set to the total transfer amount
    assert_eq!(
        flow_slot.flow_out,
        transfer_amount + second_transfer_amount,
        "flow_out doesn't match expected value"
    );
    assert_eq!(
        flow_slot.flow_in, 0,
        "flow_in should be 0 for outgoing transfers"
    );

    Ok(())
}

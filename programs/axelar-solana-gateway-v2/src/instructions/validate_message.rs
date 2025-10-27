use crate::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED;
use crate::{
    GatewayConfig, GatewayError, IncomingMessage, Message, MessageExecutedEvent, MessageStatus,
};
use anchor_lang::prelude::*;
use std::str::FromStr;

#[derive(Accounts)]
#[event_cpi]
#[instruction(message: Message)]
pub struct ValidateMessage<'info> {
    #[account(
        mut,
        seeds = [IncomingMessage::SEED_PREFIX, message.command_id().as_ref()],
        bump = incoming_message_pda.load()?.bump,
        // CHECK: message must be already approved
        constraint = incoming_message_pda.load()?.status.is_approved()
            @ GatewayError::MessageNotApproved,
        // CHECK: message hash must match
        constraint = incoming_message_pda.load()?.message_hash == message.hash()
            @ GatewayError::InvalidMessageHash
    )]
    pub incoming_message_pda: AccountLoader<'info, IncomingMessage>,

    /// The caller must be a PDA derived from the destination program using command_id and signing_pda_bump
    #[account(
        signer,
        constraint = validate_caller_pda(&caller, &message, &incoming_message_pda)?
            @ GatewayError::InvalidSigningPDA
    )]
    pub caller: AccountInfo<'info>,

    #[account(
        seeds = [GatewayConfig::SEED_PREFIX],
        bump = gateway_root_pda.load()?.bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,
}

pub fn validate_message_handler(ctx: Context<ValidateMessage>, message: Message) -> Result<()> {
    let incoming_message_pda = &mut ctx.accounts.incoming_message_pda.load_mut()?;
    incoming_message_pda.status = MessageStatus::executed();

    // Parse destination address
    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_| GatewayError::InvalidDestinationAddress)?;

    let command_id = message.command_id();
    let cc_id = &message.cc_id;

    emit_cpi!(MessageExecutedEvent {
        command_id,
        destination_address,
        payload_hash: message.payload_hash,
        source_chain: cc_id.chain.clone(),
        cc_id: cc_id.id.clone(),
        source_address: message.source_address.clone(),
        destination_chain: message.destination_chain.clone(),
    });

    Ok(())
}

fn validate_caller_pda(
    caller: &AccountInfo,
    message: &Message,
    incoming_message: &AccountLoader<'_, IncomingMessage>,
) -> Result<bool> {
    use std::str::FromStr;

    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_| GatewayError::InvalidDestinationAddress)?;

    let command_id = message.command_id();

    // Pubkey::create_program_address(&[prefix, command_id, &[signing_pda_bump]], destination_address)
    // each message has its own signing pda for a given executable
    let expected_signing_pda = Pubkey::create_program_address(
        &[
            VALIDATE_MESSAGE_SIGNING_SEED,
            command_id.as_ref(),
            &[incoming_message.load()?.signing_pda_bump],
        ],
        &destination_address,
    )
    .map_err(|_| GatewayError::InvalidSigningPDA)?;

    Ok(caller.key == &expected_signing_pda)
}

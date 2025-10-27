use anchor_lang::{prelude::*, solana_program, InstructionData};
use solana_program::instruction::Instruction;

use crate::program::AxelarSolanaGovernanceV2;
use crate::{ExecutableProposal, ExecuteProposalCallData, GovernanceConfig, GovernanceError};
use axelar_solana_gateway_v2::{executable::*, executable_accounts};
use governance_gmp::{GovernanceCommand, GovernanceCommandPayload};

executable_accounts!(ProcessGmp);

#[derive(Accounts)]
pub struct ProcessGmp<'info> {
    // GMP Accounts
    pub executable: AxelarExecuteAccounts<'info>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub payer: Signer<'info>,

    // Even though governance_config doesn't need to be mutable in
    // all GMP instructions, we make it mutable here to simplify CPI handling.
    // This avoids manual account construction for each instruction.
    // TODO: recheck this choice later
    #[account(
    	mut,
    	seeds = [GovernanceConfig::SEED_PREFIX],
        bump = governance_config.bump
    )]
    pub governance_config: Account<'info, GovernanceConfig>,

    // Variable accounts are kept as unchecked. We self-CPI and check them for each separate instruction
    #[account(mut)]
    pub proposal_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub operator_proposal_pda: UncheckedAccount<'info>,

    #[account(
        seeds = [b"__event_authority"],
        bump,
    )]
    pub governance_event_authority: SystemAccount<'info>,

    pub axelar_governance_program: Program<'info, AxelarSolanaGovernanceV2>,
}

pub fn process_gmp_handler(
    ctx: Context<ProcessGmp>,
    message: Message,
    payload: Vec<u8>,
) -> Result<()> {
    // Ensure the message is from an authorized address and chain.
    // Check this first before validating message,
    // to avoid copying Message + it's a cheaper check
    ensure_authorized_gmp_command(&ctx.accounts.governance_config, &message)?;

    validate_message_raw(&ctx.accounts.axelar_executable(), message, &payload)?;

    let (cmd_payload, _, _, proposal_hash) = calculate_gmp_context(payload)?;

    process_proposal_gmp(ctx, proposal_hash, cmd_payload)
}

fn ensure_authorized_gmp_command(config: &GovernanceConfig, message: &Message) -> Result<()> {
    // Ensure the incoming address matches stored configuration.
    let address_hash =
        solana_program::keccak::hashv(&[message.source_address.as_bytes()]).to_bytes();
    if address_hash != config.address_hash {
        msg!(
            "Incoming governance GMP message came with non authorized address: {}",
            message.source_address
        );
        return err!(GovernanceError::UnauthorizedAddress);
    }

    // Ensure the incoming chain matches stored configuration.
    let chain_hash = solana_program::keccak::hashv(&[message.cc_id.chain.as_bytes()]).to_bytes();
    if chain_hash != config.chain_hash {
        msg!(
            "Incoming governance GMP message came with non authorized chain: {}",
            message.cc_id.chain
        );
        return err!(GovernanceError::UnauthorizedChain);
    }

    Ok(())
}

fn calculate_gmp_context(
    payload: Vec<u8>,
) -> Result<(
    GovernanceCommandPayload,
    Pubkey,
    ExecuteProposalCallData,
    [u8; 32],
)> {
    let cmd_payload = crate::payload_conversions::decode_payload(&payload)?;

    let target = crate::payload_conversions::decode_payload_target(&cmd_payload.target)?;

    let execute_proposal_call_data =
        crate::payload_conversions::decode_payload_call_data(&cmd_payload.call_data)?;

    let proposal_hash = ExecutableProposal::calculate_hash(
        &target,
        &execute_proposal_call_data,
        &cmd_payload.native_value.to_le_bytes(),
    );

    Ok((
        cmd_payload,
        target,
        execute_proposal_call_data,
        proposal_hash,
    ))
}

fn process_proposal_gmp(
    ctx: Context<ProcessGmp>,
    proposal_hash: [u8; 32],
    cmd_payload: GovernanceCommandPayload,
) -> Result<()> {
    match cmd_payload.command {
        GovernanceCommand::ScheduleTimeLockProposal => {
            msg!("Processing ScheduleTimeLockProposal via GMP");
            schedule_timelock_proposal(ctx, cmd_payload, proposal_hash)
        }
        GovernanceCommand::CancelTimeLockProposal => {
            msg!("Processing CancelTimeLockProposal via GMP");
            cancel_timelock_proposal(ctx, cmd_payload, proposal_hash)
        }
        GovernanceCommand::ApproveOperatorProposal => {
            msg!("Processing ApproveOperatorProposal via GMP");
            approve_operator_proposal(ctx, cmd_payload, proposal_hash)
        }
        GovernanceCommand::CancelOperatorApproval => {
            msg!("Processing CancelOperatorApproval via GMP");
            cancel_operator_proposal(ctx, cmd_payload, proposal_hash)
        }
        _ => {
            msg!("Governance GMP command cannot be processed");
            err!(GovernanceError::InvalidInstructionData)
        }
    }
}

#[allow(clippy::type_complexity)]
fn extract_proposal_data(
    cmd_payload: GovernanceCommandPayload,
) -> Result<(u64, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let eta = cmd_payload
        .eta
        .try_into()
        .map_err(|_| GovernanceError::InvalidInstructionData)?;
    let native_value = cmd_payload.native_value.to_le_bytes::<32>().to_vec();
    let call_data = cmd_payload.call_data.into();
    let target = cmd_payload.target.to_vec();

    Ok((eta, native_value, call_data, target))
}

fn schedule_timelock_proposal(
    ctx: Context<ProcessGmp>,
    cmd_payload: GovernanceCommandPayload,
    proposal_hash: [u8; 32],
) -> Result<()> {
    let (eta, native_value, call_data, target) = extract_proposal_data(cmd_payload)?;

    let instruction_data = crate::instruction::ScheduleTimelockProposal {
        proposal_hash,
        eta,
        target,
        native_value,
        call_data,
    };

    let schedule_instruction = Instruction {
        program_id: crate::ID,
        accounts: crate::accounts::ScheduleTimelockProposal {
            system_program: ctx.accounts.system_program.key(),
            governance_config: ctx.accounts.governance_config.key(),
            payer: ctx.accounts.payer.key(),
            proposal_pda: ctx.accounts.proposal_pda.key(),
            // for event cpi
            event_authority: ctx.accounts.governance_event_authority.key(),
            program: ctx.accounts.axelar_governance_program.key(),
        }
        .to_account_metas(None),
        data: instruction_data.data(),
    };

    let account_infos =
        crate::__cpi_client_accounts_schedule_timelock_proposal::ScheduleTimelockProposal {
            system_program: ctx.accounts.system_program.to_account_info(),
            governance_config: ctx.accounts.governance_config.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            proposal_pda: ctx.accounts.proposal_pda.to_account_info(),
            // for emit cpi
            event_authority: ctx.accounts.governance_event_authority.to_account_info(),
            program: ctx.accounts.axelar_governance_program.to_account_info(),
        }
        .to_account_infos();

    invoke_signed_with_governance_config(
        &schedule_instruction,
        &account_infos,
        ctx.accounts.governance_config.bump,
    )
}

fn cancel_timelock_proposal(
    ctx: Context<ProcessGmp>,
    cmd_payload: GovernanceCommandPayload,
    proposal_hash: [u8; 32],
) -> Result<()> {
    let (eta, native_value, call_data, target) = extract_proposal_data(cmd_payload)?;

    let instruction_data = crate::instruction::CancelTimelockProposal {
        proposal_hash,
        eta,
        native_value,
        call_data,
        target,
    };

    // Create the instruction with accounts matching CancelTimelockProposal
    let cancel_instruction = Instruction {
        program_id: crate::ID,
        accounts: crate::accounts::CancelTimelockProposal {
            governance_config: ctx.accounts.governance_config.key(),
            proposal_pda: ctx.accounts.proposal_pda.key(),
            event_authority: ctx.accounts.governance_event_authority.key(),
            program: ctx.accounts.axelar_governance_program.key(),
        }
        .to_account_metas(None),
        data: instruction_data.data(),
    };

    let account_infos =
        crate::__cpi_client_accounts_cancel_timelock_proposal::CancelTimelockProposal {
            governance_config: ctx.accounts.governance_config.to_account_info(),
            proposal_pda: ctx.accounts.proposal_pda.to_account_info(),
            event_authority: ctx.accounts.governance_event_authority.to_account_info(),
            program: ctx.accounts.axelar_governance_program.to_account_info(),
        }
        .to_account_infos();

    invoke_signed_with_governance_config(
        &cancel_instruction,
        &account_infos,
        ctx.accounts.governance_config.bump,
    )
}

fn approve_operator_proposal(
    ctx: Context<ProcessGmp>,
    cmd_payload: GovernanceCommandPayload,
    proposal_hash: [u8; 32],
) -> Result<()> {
    let (_, native_value, call_data, target) = extract_proposal_data(cmd_payload)?;

    let instruction_data = crate::instruction::ApproveOperatorProposal {
        proposal_hash,
        native_value,
        call_data,
        target,
    };

    // Create the instruction with accounts matching ApproveOperatorProposal
    let approve_instruction = Instruction {
        program_id: crate::ID,
        accounts: crate::accounts::ApproveOperatorProposal {
            system_program: ctx.accounts.system_program.key(),
            governance_config: ctx.accounts.governance_config.key(),
            payer: ctx.accounts.payer.key(),
            proposal_pda: ctx.accounts.proposal_pda.key(),
            operator_proposal_pda: ctx.accounts.operator_proposal_pda.key(),
            // for event cpi
            event_authority: ctx.accounts.governance_event_authority.key(),
            program: ctx.accounts.axelar_governance_program.key(),
        }
        .to_account_metas(None),
        data: instruction_data.data(),
    };

    let account_infos =
        crate::__cpi_client_accounts_approve_operator_proposal::ApproveOperatorProposal {
            system_program: ctx.accounts.system_program.to_account_info(),
            governance_config: ctx.accounts.governance_config.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            proposal_pda: ctx.accounts.proposal_pda.to_account_info(),
            operator_proposal_pda: ctx.accounts.operator_proposal_pda.to_account_info(),
            event_authority: ctx.accounts.governance_event_authority.to_account_info(),
            program: ctx.accounts.axelar_governance_program.to_account_info(),
        }
        .to_account_infos();

    invoke_signed_with_governance_config(
        &approve_instruction,
        &account_infos,
        ctx.accounts.governance_config.bump,
    )
}

fn cancel_operator_proposal(
    ctx: Context<ProcessGmp>,
    cmd_payload: GovernanceCommandPayload,
    proposal_hash: [u8; 32],
) -> Result<()> {
    let (_, native_value, call_data, target) = extract_proposal_data(cmd_payload)?;

    let instruction_data = crate::instruction::CancelOperatorProposal {
        proposal_hash,
        native_value,
        call_data,
        target,
    };

    // Create the instruction with accounts matching CancelOperatorProposal
    let cancel_instruction = Instruction {
        program_id: crate::ID,
        accounts: crate::accounts::CancelOperatorProposal {
            governance_config: ctx.accounts.governance_config.key(),
            proposal_pda: ctx.accounts.proposal_pda.key(),
            operator_proposal_pda: ctx.accounts.operator_proposal_pda.key(),
            // for event cpi
            event_authority: ctx.accounts.governance_event_authority.key(),
            program: ctx.accounts.axelar_governance_program.key(),
        }
        .to_account_metas(None),
        data: instruction_data.data(),
    };

    let account_infos =
        crate::__cpi_client_accounts_cancel_operator_proposal::CancelOperatorProposal {
            governance_config: ctx.accounts.governance_config.to_account_info(),
            proposal_pda: ctx.accounts.proposal_pda.to_account_info(),
            operator_proposal_pda: ctx.accounts.operator_proposal_pda.to_account_info(),
            event_authority: ctx.accounts.governance_event_authority.to_account_info(),
            program: ctx.accounts.axelar_governance_program.to_account_info(),
        }
        .to_account_infos();

    invoke_signed_with_governance_config(
        &cancel_instruction,
        &account_infos,
        ctx.accounts.governance_config.bump,
    )
}

fn invoke_signed_with_governance_config(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    governance_config_bump: u8,
) -> Result<()> {
    let seeds = &[GovernanceConfig::SEED_PREFIX, &[governance_config_bump]];
    let signer_seeds = &[&seeds[..]];

    solana_program::program::invoke_signed(instruction, account_infos, signer_seeds)?;
    Ok(())
}

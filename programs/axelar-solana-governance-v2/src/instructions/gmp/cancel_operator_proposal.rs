use crate::{ExecutableProposal, GovernanceConfig, OperatorProposal, OperatorProposalCancelled};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[event_cpi]
#[instruction(proposal_hash: [u8; 32], native_value: Vec<u8>, target: Vec<u8>, call_data: Vec<u8>)]
pub struct CancelOperatorProposal<'info> {
    #[account(
        mut,
        signer,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump = governance_config.bump,
    )]
    pub governance_config: Account<'info, GovernanceConfig>,

    #[account(
        seeds = [ExecutableProposal::SEED_PREFIX, &proposal_hash],
        bump = proposal_pda.bump
    )]
    pub proposal_pda: Account<'info, ExecutableProposal>,

    #[account(
        mut,
        close = governance_config,
        seeds = [OperatorProposal::SEED_PREFIX, &proposal_hash],
        bump = proposal_pda.managed_bump
    )]
    pub operator_proposal_pda: Account<'info, OperatorProposal>,
}

pub fn cancel_operator_proposal_handler(
    ctx: Context<CancelOperatorProposal>,
    proposal_hash: [u8; 32],
    native_value: Vec<u8>,
    target: Vec<u8>,
    call_data: Vec<u8>,
) -> Result<()> {
    emit_cpi!(OperatorProposalCancelled {
        hash: proposal_hash,
        target_address: target,
        call_data,
        native_value,
    });

    Ok(())
}

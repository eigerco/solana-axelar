use crate::{
    ExecutableProposal, GovernanceConfig, GovernanceError, OperatorProposal,
    OperatorProposalApproved,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[event_cpi]
#[instruction(proposal_hash: [u8; 32], native_value: Vec<u8>, target: Vec<u8>, call_data: Vec<u8>)]
pub struct ApproveOperatorProposal<'info> {
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
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
        init,
        payer = payer,
        space = OperatorProposal::DISCRIMINATOR.len() + OperatorProposal::INIT_SPACE,
        seeds = [OperatorProposal::SEED_PREFIX, &proposal_hash],
        bump,
    )]
    pub operator_proposal_pda: Account<'info, OperatorProposal>,
}

pub fn approve_operator_proposal_handler(
    ctx: Context<ApproveOperatorProposal>,
    proposal_hash: [u8; 32],
    native_value: Vec<u8>,
    target: Vec<u8>,
    call_data: Vec<u8>,
) -> Result<()> {
    if ctx.accounts.proposal_pda.managed_bump != ctx.bumps.operator_proposal_pda {
        return err!(GovernanceError::InvalidArgument);
    }

    emit_cpi!(OperatorProposalApproved {
        hash: proposal_hash,
        target_address: target,
        call_data,
        native_value,
    });

    Ok(())
}

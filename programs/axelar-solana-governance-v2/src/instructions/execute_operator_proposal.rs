use crate::{
    check_governance_config_presence, check_target_program_presence, execute_proposal_cpi,
    ExecutableProposal, ExecuteProposalData, GovernanceConfig, GovernanceError, OperatorProposal,
    OperatorProposalExecuted,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[event_cpi]
#[instruction(execute_proposal_data: ExecuteProposalData)]
pub struct ExecuteOperatorProposal<'info> {
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump = governance_config.bump,
    )]
    pub governance_config: Account<'info, GovernanceConfig>,

    #[account(
        mut,
        close = governance_config,
        seeds = [
            ExecutableProposal::SEED_PREFIX,
            &ExecutableProposal::hash_from_data(&execute_proposal_data),
        ],
        bump = proposal_pda.bump
    )]
    pub proposal_pda: Account<'info, crate::ExecutableProposal>,

    /// The operator account that must sign this transaction
    #[account(
        constraint = operator.key().to_bytes() == governance_config.operator
        	@ GovernanceError::UnauthorizedOperator
    )]
    pub operator: Signer<'info>,

    #[account(
        mut,
        close = governance_config,
        seeds = [
        	OperatorProposal::SEED_PREFIX,
        	&ExecutableProposal::hash_from_data(&execute_proposal_data),
        ],
        bump
    )]
    pub operator_pda_marker_account: Account<'info, crate::OperatorProposal>,
}

pub fn execute_operator_proposal_handler(
    ctx: Context<ExecuteOperatorProposal>,
    execute_proposal_data: ExecuteProposalData,
) -> Result<()> {
    let target_program = Pubkey::new_from_array(execute_proposal_data.target_address);

    // Note: No ETA validation for operator proposals - they can be executed immediately
    let remaining_accounts = ctx.remaining_accounts;

    check_governance_config_presence(
        &ctx.accounts.governance_config.key(),
        remaining_accounts,
        &execute_proposal_data.call_data.solana_accounts,
    )?;

    check_target_program_presence(remaining_accounts, &target_program)?;

    let governance_config_bump = ctx.accounts.governance_config.bump;

    execute_proposal_cpi(
        &execute_proposal_data,
        remaining_accounts,
        &ctx.accounts.governance_config,
        governance_config_bump,
    )?;

    let proposal_hash = ExecutableProposal::calculate_hash(
        &target_program,
        &execute_proposal_data.call_data,
        &execute_proposal_data.native_value,
    );

    emit_cpi!(OperatorProposalExecuted {
        hash: proposal_hash,
        target_address: execute_proposal_data.target_address.to_vec(),
        call_data: execute_proposal_data.call_data.call_data,
        native_value: execute_proposal_data.native_value.to_vec(),
    });

    Ok(())
}

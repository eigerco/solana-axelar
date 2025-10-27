use crate::{GovernanceConfig, GovernanceError, OperatorshipTransferred};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[event_cpi]
/// Either the operator_account is a signer and matches the stored operator,
/// or the governance_config account itself is a signer
pub struct TransferOperatorship<'info> {
    pub system_program: Program<'info, System>,

    /// The current operator account, optional
    #[account(
        constraint = operator_account.key.to_bytes() == governance_config.operator
            @ GovernanceError::MissingRequiredSignature,
    )]
    pub operator_account: Option<Signer<'info>>,

    #[account(
        mut,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump = governance_config.bump,
        constraint = operator_account.is_some() || governance_config.to_account_info().is_signer
            @ GovernanceError::MissingRequiredSignature,
    )]
    pub governance_config: Account<'info, GovernanceConfig>,
}

pub fn transfer_operatorship_handler(
    ctx: Context<TransferOperatorship>,
    new_operator: [u8; 32],
) -> Result<()> {
    let config = &mut ctx.accounts.governance_config;

    let old_operator = config.operator;
    config.operator = new_operator;

    emit_cpi!(OperatorshipTransferred {
        old_operator: old_operator.to_vec(),
        new_operator: new_operator.to_vec(),
    });

    Ok(())
}

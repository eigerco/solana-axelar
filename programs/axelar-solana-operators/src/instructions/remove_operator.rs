use crate::state::*;
use crate::ErrorCode;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct RemoveOperator<'info> {
    #[account(
        mut,
        address = registry.owner @ ErrorCode::UnauthorizedOwner
    )]
    pub owner: Signer<'info>,

    /// CHECK: Referenced in operator_account
    pub operator_to_remove: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [OperatorRegistry::SEED_PREFIX],
        bump = registry.bump,
    )]
    pub registry: Account<'info, OperatorRegistry>,

    #[account(
        mut,
        close = owner,
        seeds = [
            OperatorAccount::SEED_PREFIX,
            operator_to_remove.key().as_ref(),
        ],
        bump = operator_account.bump,
        constraint = operator_account.operator == operator_to_remove.key() @ ErrorCode::InvalidOperator
    )]
    pub operator_account: Account<'info, OperatorAccount>,
}

pub fn remove_operator(ctx: Context<RemoveOperator>) -> Result<()> {
    let registry = &mut ctx.accounts.registry;
    registry.operator_count = registry
        .operator_count
        .checked_sub(1)
        // Should never happen if the Operator PDA exists
        .ok_or_else(|| -> Error { ProgramError::InvalidAccountData.into() })?;

    Ok(())
}

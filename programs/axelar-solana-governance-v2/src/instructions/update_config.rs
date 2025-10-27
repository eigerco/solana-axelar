use crate::{GovernanceConfig, GovernanceConfigUpdate, GovernanceError};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        constraint = governance_config.operator == payer.key().to_bytes()
            @ GovernanceError::NotOperator
    )]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump
    )]
    pub governance_config: Account<'info, GovernanceConfig>,
}

pub fn update_config_handler(
    ctx: Context<UpdateConfig>,
    params: GovernanceConfigUpdate,
) -> Result<()> {
    let config = &mut ctx.accounts.governance_config;
    config.update(params)
}

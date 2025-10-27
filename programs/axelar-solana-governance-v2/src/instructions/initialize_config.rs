use crate::{GovernanceConfig, GovernanceConfigInit, GovernanceError};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub upgrade_authority: Signer<'info>,

    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = anchor_lang::solana_program::bpf_loader_upgradeable::ID,
        constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()) @ GovernanceError::InvalidUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,

    #[account(
        init,
        payer = payer,
        space = GovernanceConfig::DISCRIMINATOR.len() + GovernanceConfig::INIT_SPACE,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump
    )]
    pub governance_config: Account<'info, GovernanceConfig>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_config_handler(
    ctx: Context<InitializeConfig>,
    params: GovernanceConfigInit,
) -> Result<()> {
    let config = &mut ctx.accounts.governance_config;

    // Initialize account data
    config.bump = ctx.bumps.governance_config;
    config.chain_hash = params.chain_hash;
    config.address_hash = params.address_hash;
    config.minimum_proposal_eta_delay = params.minimum_proposal_eta_delay;
    config.operator = params.operator;

    // Validate the config
    config.validate_config()?;

    Ok(())
}

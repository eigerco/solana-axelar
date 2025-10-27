#![allow(clippy::missing_asserts_for_indexing)]
use crate::seed_prefixes::{GATEWAY_SEED, VERIFIER_SET_TRACKER_SEED};
use crate::{
    state::config::{GatewayConfig, InitializeConfigParams},
    u256::U256,
    GatewayError, VerifierSetTracker,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(params: InitializeConfigParams)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub upgrade_authority: Signer<'info>,

    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        // CHECK: correct program_data pda is given
        seeds::program = anchor_lang::solana_program::bpf_loader_upgradeable::ID,
        // CHECK: upgrade authority in program_data matches the one passed as signer
        constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()) @ GatewayError::InvalidUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,

    /// The gateway configuration PDA being initialized
    #[account(
        init,
        payer = payer,
        space = GatewayConfig::DISCRIMINATOR.len() + std::mem::size_of::<GatewayConfig>(),
        seeds = [GATEWAY_SEED],
        bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,

    pub system_program: Program<'info, System>,

    #[account(
	    init,
	    payer = payer,
	    space = VerifierSetTracker::DISCRIMINATOR.len() + std::mem::size_of::<VerifierSetTracker>(),
	    seeds = [
	        VERIFIER_SET_TRACKER_SEED,
	        params.initial_verifier_set.hash.as_slice()
	    ],
	    bump
	)]
    pub verifier_set_tracker_pda: AccountLoader<'info, VerifierSetTracker>,
}

#[allow(clippy::cast_sign_loss)]
pub fn initialize_config_handler(
    ctx: Context<InitializeConfig>,
    params: InitializeConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.gateway_root_pda.load_init()?;

    // Initialize GatewayConfig (i.e. gateway config pda) state
    config.current_epoch = U256::from(1);
    config.previous_verifier_set_retention = params.previous_verifier_retention;
    config.minimum_rotation_delay = params.minimum_rotation_delay;
    config.last_rotation_timestamp = Clock::get()?.unix_timestamp as u64;
    config.operator = params.operator;
    config.domain_separator = params.domain_separator;
    config.bump = ctx.bumps.gateway_root_pda;

    let set_tracker = &mut ctx.accounts.verifier_set_tracker_pda.load_init()?;

    // Initialize verifier set tracker pda state
    set_tracker.bump = ctx.bumps.verifier_set_tracker_pda;
    set_tracker.epoch = U256::from(1);
    set_tracker.verifier_set_hash = params.initial_verifier_set.hash;

    Ok(())
}

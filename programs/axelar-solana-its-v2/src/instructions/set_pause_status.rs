use crate::state::InterchainTokenService;
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_lang::solana_program::bpf_loader_upgradeable;

#[derive(Accounts)]
#[instruction(paused: bool)]
pub struct SetPauseStatus<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = bpf_loader_upgradeable::ID,
        constraint = program_data.upgrade_authority_address == Some(payer.key())
            @ ProgramError::InvalidAccountData
    )]
    pub program_data: Account<'info, ProgramData>,

    #[account(
    	mut,
     	seeds = [InterchainTokenService::SEED_PREFIX],
     	bump = its_root_pda.bump,
      	// TODO(v2) check if this is necessary as it differs from v1
      	// Check that the paused status is actually changing
      	constraint = its_root_pda.paused != paused @ ProgramError::InvalidArgument,
    )]
    pub its_root_pda: Account<'info, InterchainTokenService>,
    //
    // TODO(v2) v1 has system_program here but it is not
    // necessary since the PDA size isn't changing.
    // You can remove this comment after the migration has been reverified.
    // pub system_program: Program<'info, System>,
}

pub fn set_pause_status(ctx: Context<SetPauseStatus>, paused: bool) -> Result<()> {
    ctx.accounts.its_root_pda.paused = paused;

    Ok(())
}

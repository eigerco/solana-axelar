use crate::state::{InterchainTokenService, Roles, UserRoles};
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_lang::solana_program::bpf_loader_upgradeable;

/// Initialize the configuration PDA.
#[derive(Accounts)]
pub struct Initialize<'info> {
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
    	init,
      	payer = payer,
     	space = InterchainTokenService::DISCRIMINATOR.len() + InterchainTokenService::INIT_SPACE,
     	seeds = [InterchainTokenService::SEED_PREFIX],
     	bump,
    )]
    pub its_root_pda: Account<'info, InterchainTokenService>,

    pub system_program: Program<'info, System>,

    pub operator: Signer<'info>,

    /// The address of the account that will store the roles of the operator account.
    #[account(
    	init,
	 	payer = payer,
	 	space = UserRoles::DISCRIMINATOR.len() + UserRoles::INIT_SPACE,
	 	seeds = [
			UserRoles::SEED_PREFIX,
			its_root_pda.key().as_ref(),
			operator.key().as_ref(),
		],
	 	bump,
    )]
    pub user_roles_account: Account<'info, UserRoles>,
}

pub fn initialize(
    ctx: Context<Initialize>,
    chain_name: String,
    its_hub_address: String,
) -> Result<()> {
    // Initialize ITS root
    *ctx.accounts.its_root_pda =
        InterchainTokenService::new(ctx.bumps.its_root_pda, chain_name, its_hub_address);

    // Initialize and assign OPERATOR role to the operator account.
    ctx.accounts.user_roles_account.roles = Roles::OPERATOR;
    ctx.accounts.user_roles_account.bump = ctx.bumps.user_roles_account;

    Ok(())
}

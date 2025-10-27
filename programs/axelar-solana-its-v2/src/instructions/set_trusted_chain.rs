use crate::{
    events::TrustedChainSet,
    state::{InterchainTokenService, Roles, RolesError, UserRoles},
};
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_lang::solana_program::bpf_loader_upgradeable;

#[event_cpi]
#[derive(Accounts)]
#[instruction(chain_name: String)]
pub struct SetTrustedChain<'info> {
    /// Payer must be either the program upgrade authority or have the OPERATOR role.
    #[account(mut,
    	constraint =
     		user_roles.as_ref().is_some() || program_data.as_ref()
       			.is_some_and(|pd| pd.upgrade_authority_address == Some(payer.key()))
      	 		@ ProgramError::MissingRequiredSignature,
    )]
    pub payer: Signer<'info>,

    /// The address of the account that will store the roles of the operator account.
    #[account(
	 	seeds = [
			UserRoles::SEED_PREFIX,
			its_root_pda.key().as_ref(),
			payer.key().as_ref(),
		],
	 	bump = user_roles.bump,
		// Require the payer to have the OPERATOR role.
		constraint = user_roles.roles.contains(Roles::OPERATOR) @ RolesError::MissingOperatorRole,
    )]
    pub user_roles: Option<Account<'info, UserRoles>>,

    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = bpf_loader_upgradeable::ID,
        constraint = program_data.upgrade_authority_address == Some(payer.key())
            @ ProgramError::InvalidAccountData,
    )]
    pub program_data: Option<Account<'info, ProgramData>>,

    #[account(
    	mut,
    	realloc = InterchainTokenService::space(its_root_pda.trusted_chains.len() + 1),
     	realloc::payer = payer,
      	realloc::zero = false,
     	seeds = [InterchainTokenService::SEED_PREFIX],
     	bump = its_root_pda.bump,
      	// Ensure the chain is not already added.
      	constraint = !its_root_pda.is_trusted_chain(chain_name) @ ProgramError::InvalidArgument,
    )]
    pub its_root_pda: Account<'info, InterchainTokenService>,

    pub system_program: Program<'info, System>,
}

/// Sets a new trusted chain in the ITS configuration.
/// To authorize this action, the payer must be either the program upgrade authority
/// or have the OPERATOR role.
///
/// If both accounts are passed, the payer must be the program upgrade authority *and*
/// have the OPERATOR role.
pub fn set_trusted_chain(ctx: Context<SetTrustedChain>, chain_name: String) -> Result<()> {
    ctx.accounts
        .its_root_pda
        .add_trusted_chain(chain_name.clone());

    emit_cpi!(TrustedChainSet { chain_name });

    Ok(())
}

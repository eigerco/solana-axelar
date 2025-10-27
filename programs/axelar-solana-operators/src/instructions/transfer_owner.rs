use crate::state::*;
use crate::ErrorCode;
use crate::OwnershipTransferred;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TransferOwner<'info> {
    #[account(
        mut,
        address = registry.owner @ ErrorCode::UnauthorizedOwner
    )]
    pub owner: Signer<'info>,

    /// CHECK: The new owner pubkey
    #[account(
    	// Ensure the new owner is not the same as the current owner
		constraint = new_owner.key() != registry.owner @ ErrorCode::SameMaster
    )]
    pub new_owner: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [OperatorRegistry::SEED_PREFIX],
        bump = registry.bump,
    )]
    pub registry: Account<'info, OperatorRegistry>,
}

pub fn transfer_owner(ctx: Context<TransferOwner>) -> Result<()> {
    let registry = &mut ctx.accounts.registry;

    // Update the owner
    registry.owner = ctx.accounts.new_owner.key();

    emit!(OwnershipTransferred {
        old_owner: ctx.accounts.owner.key(),
        new_owner: ctx.accounts.new_owner.key(),
    });

    Ok(())
}

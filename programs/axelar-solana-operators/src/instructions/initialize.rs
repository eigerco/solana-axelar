use crate::state::*;
use anchor_lang::prelude::*;

/// Initialize the operator registry
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = OperatorRegistry::DISCRIMINATOR.len() + OperatorRegistry::INIT_SPACE,
        seeds = [OperatorRegistry::SEED_PREFIX],
        bump,
    )]
    pub registry: Account<'info, OperatorRegistry>,

    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let registry = &mut ctx.accounts.registry;

    registry.owner = ctx.accounts.owner.key();
    registry.operator_count = 0;
    registry.bump = ctx.bumps.registry;

    Ok(())
}

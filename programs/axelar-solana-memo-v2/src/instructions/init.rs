use anchor_lang::prelude::*;

use crate::Counter;

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // The counter account
    #[account(
        init,
        space = Counter::DISCRIMINATOR.len() + Counter::INIT_SPACE,
        payer = payer,
        seeds = [Counter::SEED_PREFIX],
        bump
    )]
    pub counter: Account<'info, Counter>,

    pub system_program: Program<'info, System>,
}

pub fn init_handler(ctx: Context<Init>) -> Result<()> {
    ctx.accounts.counter.bump = ctx.bumps.counter;

    Ok(())
}

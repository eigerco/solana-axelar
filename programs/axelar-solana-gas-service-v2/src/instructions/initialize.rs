use crate::state::Treasury;
use anchor_lang::prelude::*;
use axelar_solana_operators::OperatorAccount;

/// Initialize the configuration PDA.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub operator: Signer<'info>,

    #[account(
        seeds = [
            OperatorAccount::SEED_PREFIX,
            operator.key().as_ref(),
        ],
        bump = operator_pda.bump,
        seeds::program = axelar_solana_operators::ID
    )]
    pub operator_pda: Account<'info, OperatorAccount>,

    pub system_program: Program<'info, System>,

    #[account(
        init,
        space = Treasury::DISCRIMINATOR.len() + Treasury::INIT_SPACE,
        payer = payer,
        seeds = [
            Treasury::SEED_PREFIX,
        ],
        bump,
    )]
    pub treasury: AccountLoader<'info, Treasury>,
}

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury.load_init()?;
    treasury.bump = ctx.bumps.treasury;

    Ok(())
}

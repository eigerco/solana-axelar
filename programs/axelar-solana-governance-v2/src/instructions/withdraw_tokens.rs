use crate::GovernanceConfig;
use anchor_lang::prelude::*;
use program_utils::transfer_lamports_anchor;

#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        signer,
        seeds = [GovernanceConfig::SEED_PREFIX],
        bump = governance_config.bump,
    )]
    pub governance_config: Account<'info, GovernanceConfig>,

    /// The account that will receive the withdrawn lamports
    /// CHECK: This can be any account that should receive the funds
    #[account(mut)]
    pub receiver: AccountInfo<'info>,
}

// Note: this should be called by the governance through a proposal
pub fn withdraw_tokens_handler(ctx: Context<WithdrawTokens>, amount: u64) -> Result<()> {
    let governance_config = &ctx.accounts.governance_config;
    let receiver = &ctx.accounts.receiver;

    let governance_account_info = governance_config.to_account_info();

    // Note: We need manual lamport transfer because we are dealing with
    // governance_config which is a data account
    transfer_lamports_anchor!(governance_account_info, receiver, amount);

    msg!(
        "{} lamports were transferred from {}",
        amount,
        governance_config.key()
    );
    msg!("{} lamports were transferred to {}", amount, receiver.key());

    Ok(())
}

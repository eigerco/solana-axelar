use crate::events::GasRefundedEvent;
use crate::state::Treasury;
use anchor_lang::prelude::*;
use axelar_solana_operators::OperatorAccount;
use program_utils::transfer_lamports_anchor;

/// Refund previously collected native SOL fees (operator only).
#[event_cpi]
#[derive(Accounts)]
pub struct RefundFees<'info> {
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

    /// CHECK: Can be any account to receive funds
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            Treasury::SEED_PREFIX,
        ],
        bump = treasury.load()?.bump,
    )]
    pub treasury: AccountLoader<'info, Treasury>,
}

pub fn refund_native_fees(ctx: Context<RefundFees>, message_id: String, amount: u64) -> Result<()> {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData.into());
    }

    transfer_lamports_anchor!(
        ctx.accounts.treasury.to_account_info(),
        ctx.accounts.receiver.to_account_info(),
        amount
    );

    emit_cpi!(GasRefundedEvent {
        receiver: ctx.accounts.receiver.key(),
        message_id,
        amount,
        spl_token_account: None,
    });

    Ok(())
}

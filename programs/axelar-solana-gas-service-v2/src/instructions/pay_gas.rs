use crate::events::GasPaidEvent;
use crate::state::Treasury;
use anchor_lang::{prelude::*, system_program};

/// Pay gas fees for a contract call using native SOL.
#[event_cpi]
#[derive(Accounts)]
pub struct PayGas<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [
            Treasury::SEED_PREFIX,
        ],
        bump = treasury.load()?.bump,
    )]
    pub treasury: AccountLoader<'info, Treasury>,

    pub system_program: Program<'info, System>,
}

pub fn pay_gas(
    ctx: Context<PayGas>,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    amount: u64,
    refund_address: Pubkey,
) -> Result<()> {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData.into());
    }

    let sender = ctx.accounts.sender.to_account_info();
    let treasury_account_info = ctx.accounts.treasury.to_account_info();

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: sender,
                to: treasury_account_info,
            },
        ),
        amount,
    )?;

    emit_cpi!(GasPaidEvent {
        sender: ctx.accounts.sender.key(),
        destination_chain,
        destination_address,
        payload_hash,
        amount,
        refund_address,
        spl_token_account: None,
    });

    Ok(())
}

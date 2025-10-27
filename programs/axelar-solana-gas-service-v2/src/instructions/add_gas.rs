use crate::events::GasAddedEvent;
use crate::state::Treasury;
use anchor_lang::{prelude::*, system_program};

/// Add more native SOL gas to an existing transaction.
#[event_cpi]
#[derive(Accounts)]
pub struct AddGas<'info> {
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

pub fn add_gas(
    ctx: Context<AddGas>,
    message_id: String,
    amount: u64,
    refund_address: Pubkey,
) -> Result<()> {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData.into());
    }

    let payer = &ctx.accounts.sender.to_account_info();
    let treasury_account_info = &ctx.accounts.treasury.to_account_info();

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: payer.clone(),
                to: treasury_account_info.clone(),
            },
        ),
        amount,
    )?;

    emit_cpi!(GasAddedEvent {
        sender: *payer.key,
        message_id,
        amount,
        refund_address,
        spl_token_account: None,
    });

    Ok(())
}

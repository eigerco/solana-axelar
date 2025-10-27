use anchor_lang::prelude::*;
use axelar_solana_gateway_v2::{executable::*, executable_accounts};

executable_accounts!(Execute);

use crate::Counter;

#[derive(Accounts)]
pub struct Execute<'info> {
    // GMP Accounts
    pub executable: AxelarExecuteAccounts<'info>,

    // The counter account
    #[account(mut, seeds = [Counter::SEED_PREFIX], bump)]
    pub counter: Account<'info, Counter>,
}

pub fn execute_handler(
    ctx: Context<Execute>,
    message: Message,
    payload: Vec<u8>,
    encoding_scheme: axelar_solana_gateway_v2::executable::ExecutablePayloadEncodingScheme,
) -> Result<()> {
    validate_message(ctx.accounts, message, &payload, encoding_scheme)?;

    msg!("Payload size: {}", payload.len());
    let memo = std::str::from_utf8(&payload).map_err(|err| {
        msg!("Invalid UTF-8, from byte {}", err.valid_up_to());
        ProgramError::InvalidInstructionData
    })?;

    // Log memo
    log_memo(memo);

    // Increase counter
    ctx.accounts.counter.counter += 1;

    Ok(())
}

#[inline]
fn log_memo(memo: &str) {
    // If memo is longer than 10 characters, log just the first character.
    let char_count = memo.chars().count();
    if char_count > 10 {
        msg!(
            "Memo (len {}): {:?} x {} (too big to log)",
            memo.len(),
            memo.chars().next().unwrap(),
            char_count
        );
    } else {
        msg!("Memo (len {}): {:?}", memo.len(), memo);
    }
}

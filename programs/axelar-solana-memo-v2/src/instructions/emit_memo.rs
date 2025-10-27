use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct EmitMemo<'info> {
    // TODO remove unused account
    pub some_account: UncheckedAccount<'info>,
}

pub fn emit_memo_handler(_ctx: Context<EmitMemo>, message: String) -> Result<()> {
    msg!("Received memo: {}", message);
    Ok(())
}

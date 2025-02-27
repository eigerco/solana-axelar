
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;

use solana_program::pubkey::Pubkey;

use crate::state::RelayerExecutionInfo;

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_set_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    relayer_execution: RelayerExecutionInfo,
    destination_address: Pubkey,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let payer = next_account_info(accounts)?;
    let relayer_execution_info_pda = next_account_info(accounts)?;
    let executor_pda = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;

    relayer_execution.create(destination_address, relayer_execution_info_pda, executor_pda, payer, program_id, system_program)?;
    // TODO: Emit an event

    Ok(())
}

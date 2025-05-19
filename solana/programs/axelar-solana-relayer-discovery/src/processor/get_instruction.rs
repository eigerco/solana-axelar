
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;

use solana_program::pubkey::Pubkey;
use solana_program::program::set_return_data;
use solana_program::program_error::ProgramError;

use crate::state::RelayerExecutionInfo;

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_get_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_address: Pubkey
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let relayer_execution_info_pda = next_account_info(accounts)?;

    let execution_info = RelayerExecutionInfo::read(relayer_execution_info_pda, &destination_address, program_id)?;
    // TODO: Proper Errors
    let data = bincode::serialize(&execution_info).map_err(|_| ProgramError::Custom(0))?;
    set_return_data(&data);

    Ok(())
}

use axelar_solana_gas_service_events::event_prefixes;
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::log::sol_log_data;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::state::Config;
use crate::{assert_valid_config_pda, seed_prefixes};

fn ensure_valid_config_pda_ata(
    config_pda_ata: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    config_pda: &AccountInfo<'_>,
) -> ProgramResult {
    if config_pda_ata.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let ata_data =
        spl_token_2022::state::Account::unpack_from_slice(&config_pda_ata.try_borrow_data()?)?;
    if ata_data.mint != *mint.key || ata_data.owner != *config_pda.key {
        return Err(ProgramError::InvalidAccountData);
    };
    Ok(())
}

fn ensure_valid_config_pda(config_pda: &AccountInfo<'_>, program_id: &Pubkey) -> ProgramResult {
    config_pda.check_initialized_pda_without_deserialization(program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    assert_valid_config_pda(config.bump, config_pda.key)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transfer_tokens(
    token_program: &AccountInfo<'_>,
    sender_ata: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    receiver_ata: &AccountInfo<'_>,
    sender_authority: &AccountInfo<'_>,
    signer_pubkeys: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    spl_token_2022::instruction::transfer_checked(
        token_program.key,
        sender_ata.key,
        mint.key,
        receiver_ata.key,
        sender_authority.key,
        signer_pubkeys
            .iter()
            .map(|x| x.key)
            .collect::<Vec<_>>()
            .as_slice(),
        amount,
        decimals,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_pay_spl_for_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: &[u8],
    gas_fee_amount: u64,
    decimals: u8,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (accounts, signer_pubkeys) = accounts.split_at(6);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_ata = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        sender_ata,
        mint,
        config_pda_ata,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_ata.clone(),
            config_pda_ata.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_PAID_FOR_CONTRACT_CALL,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &destination_chain.into_bytes(),
        &destination_address.into_bytes(),
        &payload_hash,
        &refund_address.to_bytes(),
        params,
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn add_spl_gas(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
    decimals: u8,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (accounts, signer_pubkeys) = accounts.split_at(6);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_ata = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        sender_ata,
        mint,
        config_pda_ata,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_ata.clone(),
            config_pda_ata.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_GAS_ADDED,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &tx_hash,
        &log_index.to_le_bytes(),
        &refund_address.to_bytes(),
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn collect_fees_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let operator = next_account_info(accounts)?;
    let receiver_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    // Check: Operator matches
    if operator.key != &config.operator {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Check: Operator is signer
    if !operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        config_pda_ata,
        mint,
        receiver_account,
        config_pda,
        &[],
        amount,
        decimals,
    )?;

    invoke_signed(
        &ix,
        &[
            config_pda.clone(),
            mint.clone(),
            config_pda_ata.clone(),
            receiver_account.clone(),
            token_program.clone(),
        ],
        &[&[seed_prefixes::CONFIG_SEED, &[config.bump]]],
    )?;

    Ok(())
}

pub(crate) fn refund_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
    decimals: u8,
) -> ProgramResult {
    if fees == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let operator = next_account_info(accounts)?;
    let receiver_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    // Check: Operator matches
    if operator.key != &config.operator {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Check: Operator is signer
    if !operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        config_pda_ata,
        mint,
        receiver_account,
        config_pda,
        &[],
        fees,
        decimals,
    )?;

    invoke_signed(
        &ix,
        &[
            config_pda.clone(),
            mint.clone(),
            config_pda_ata.clone(),
            receiver_account.clone(),
            token_program.clone(),
        ],
        &[&[seed_prefixes::CONFIG_SEED, &[config.bump]]],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_GAS_REFUNDED,
        &tx_hash,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &log_index.to_le_bytes(),
        &receiver_account.key.to_bytes(),
        &fees.to_le_bytes(),
    ]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pay_spl_for_contract_call_cannot_pay_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let destination_chain = "destination_chain".to_owned();
        let destination_address = "destination_address".to_owned();
        let payload_hash = [0; 32];
        let refund_address = Pubkey::new_unique();
        let params = vec![];
        let gas_fee_amount = 0;
        let decimals = 0;

        let result = process_pay_spl_for_contract_call(
            &program_id,
            &accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            &params,
            gas_fee_amount,
            decimals,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_add_spl_gas_cannot_add_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let tx_hash = [0; 64];
        let log_index = 0;
        let gas_fee_amount = 0;
        let refund_address = Pubkey::new_unique();
        let decimals = 0;

        let result = add_spl_gas(
            &program_id,
            &accounts,
            tx_hash,
            log_index,
            gas_fee_amount,
            refund_address,
            decimals,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_collect_fees_spl_cannot_collect_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let amount = 0;
        let decimals = 0;

        let result = collect_fees_spl(&program_id, &accounts, amount, decimals);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_refund_spl_cannot_refund_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let tx_hash = [0; 64];
        let log_index = 0;
        let fees = 0;
        let decimals = 0;

        let result = refund_spl(&program_id, &accounts, tx_hash, log_index, fees, decimals);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }
}

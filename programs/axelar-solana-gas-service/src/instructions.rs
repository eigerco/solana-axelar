//! # Instruction Module
//!
//! This module provides constructors and definitions for all instructions that can be issued to the

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::system_program;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Top-level instructions supported by the Axelar Solana Gas Service program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum GasServiceInstruction {
    /// Initialize the configuration PDA.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`payer`) paying for PDA creation
    /// 1. `[]` The `operator` account of this PDA.
    /// 2. `[writable]` The `config_pda` account to be created.
    /// 3. `[]` The `system_program` account.
    Initialize,

    /// Transfer operatorship of the gas service to a new operator.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The current `operator` account
    /// 1. `[]` The new `operator` account to transfer operatorship to
    /// 2. `[writable]` The `config_pda` account
    TransferOperatorship,

    /// Use SPL tokens to pay for gas-related operations.
    SplToken(PayWithSplToken),

    /// Use SOL to pay for gas-related operations.
    Native(PayWithNativeToken),
}

/// Instructions related to paying gas fees with SPL tokens.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum PayWithSplToken {
    /// Pay gas fees for a contract call using SPL tokens.
    ForContractCall {
        /// The target blockchain (e.g., "ethereum") for the contract call.
        destination_chain: String,
        /// The recipient address on the destination chain.
        destination_address: String,
        /// A 32-byte hash representing the payload.
        payload_hash: [u8; 32],
        /// Additional parameters for the contract call.
        params: Vec<u8>,
        /// The amount of tokens to be paid as gas fees.
        gas_fee_amount: u64,
        /// The decimals for the mint
        decimals: u8,
        /// Where refunds should be sent
        refund_address: Pubkey,
    },

    /// Add more gas (SPL tokens) to an existing contract call.
    AddGas {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The additional SPL tokens to add as gas.
        gas_fee_amount: u64,
        /// The decimals for the mint
        decimals: u8,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Collect fees that have accrued in SPL tokens (operator only).
    CollectFees {
        /// The amount of SPL tokens to be collected as fees.
        amount: u64,
        /// The decimals for the mint
        decimals: u8,
    },

    /// Refund previously collected SPL token fees (operator only).
    Refund {
        /// A 64-byte unique transaction identifier
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction
        log_index: u64,
        /// The amount of SPL tokens to be refunded
        fees: u64,
        /// The decimals for the mint
        decimals: u8,
    },
}

/// Instructions related to paying gas fees with native SOL.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum PayWithNativeToken {
    /// Pay gas fees for a contract call using native SOL.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`payer`) paying the gas fee in lamports.
    /// 1. `[writable]` The `config_pda` account that receives the lamports.
    /// 2. `[]` The `system_program` account.
    ForContractCall {
        /// The target blockchain for the contract call.
        destination_chain: String,
        /// The destination address on the target chain.
        destination_address: String,
        /// A 32-byte hash representing the payload.
        payload_hash: [u8; 32],
        /// Where refunds should be sent.
        refund_address: Pubkey,
        /// Additional parameters for the contract call.
        params: Vec<u8>,
        /// The amount of SOL to pay as gas fees.
        gas_fee_amount: u64,
    },

    /// Add more native SOL gas to an existing transaction.
    ///
    /// Accounts expected:
    /// 1. `[signer, writable]` The account (`sender`) providing the additional lamports.
    /// 2. `[writable]` The `config_pda` account that receives the additional lamports.
    /// 3. `[]` The `system_program` account.
    AddGas {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The additional SOL to add as gas.
        gas_fee_amount: u64,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Collect accrued native SOL fees (operator only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `operator` account authorized to collect fees.
    /// 2. `[writable]` The `config_pda` account holding the accrued lamports to collect.
    /// 3. `[writable]` The `receiver` account where the collected lamports will be sent.
    CollectFees {
        /// The amount of SOL to collect as fees.
        amount: u64,
    },

    /// Refund previously collected native SOL fees (operator only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `operator` account authorized to issue refunds.
    /// 2. `[writable]` The `receiver` account that will receive the refunded lamports.
    /// 3. `[writable]` The `config_pda` account from which lamports are refunded.
    Refund {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The amount of SOL to be refunded.
        fees: u64,
    },
}

/// Builds an instruction to initialize the configuration PDA.
///
/// # Errors
/// - ix data cannot be serialized
pub fn init_config(payer: &Pubkey, operator: &Pubkey) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Initialize)?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to transfer operatorship of the gas service.
///
/// # Errors
/// - if the instruction could not be serialized
pub fn transfer_operatorship(
    current_operator: &Pubkey,
    new_operator: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::TransferOperatorship)?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*current_operator, true),
        AccountMeta::new_readonly(*new_operator, false),
        AccountMeta::new(config_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to pay native SOL for a contract call.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn pay_native_for_contract_call_instruction(
    payer: &Pubkey,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: Vec<u8>,
    gas_fee_amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(
        PayWithNativeToken::ForContractCall {
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            params,
            gas_fee_amount,
        },
    ))?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to add native SOL gas.
///
/// # Errors
/// - ix data cannot be serialized
pub fn add_native_gas_instruction(
    sender: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(PayWithNativeToken::AddGas {
        tx_hash,
        log_index,
        gas_fee_amount,
        refund_address,
    }))?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to collect native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn collect_native_fees_instruction(
    operator: &Pubkey,
    receiver: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(
        PayWithNativeToken::CollectFees { amount },
    ))?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new(*receiver, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to refund previously collected native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn refund_native_fees_instruction(
    operator: &Pubkey,
    receiver: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(PayWithNativeToken::Refund {
        tx_hash,
        log_index,
        fees,
    }))?;
    let (config_pda, _) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new(config_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to pay with SPL tokens for a contract call.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn pay_spl_for_contract_call_instruction(
    sender: &Pubkey,
    sender_ata: &Pubkey,
    mint: &Pubkey,
    token_program_id: &Pubkey,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: Vec<u8>,
    gas_fee_amount: u64,
    signer_pubkeys: &[Pubkey],
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::SplToken(
        PayWithSplToken::ForContractCall {
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            params,
            decimals,
            gas_fee_amount,
        },
    ))?;
    let (config_pda, _bump) = crate::get_config_pda();
    let config_pda_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &config_pda,
        mint,
        token_program_id,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*sender, true),
        AccountMeta::new(*sender_ata, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(config_pda_ata, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    for signer_pubkey in signer_pubkeys {
        accounts.push(AccountMeta::new_readonly(*signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to add SPL gas.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn add_spl_gas_instruction(
    sender: &Pubkey,
    sender_ata: &Pubkey,
    mint: &Pubkey,
    token_program_id: &Pubkey,
    signer_pubkeys: &[Pubkey],
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::SplToken(PayWithSplToken::AddGas {
        tx_hash,
        log_index,
        decimals,
        gas_fee_amount,
        refund_address,
    }))?;
    let (config_pda, _bump) = crate::get_config_pda();
    let config_pda_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &config_pda,
        mint,
        token_program_id,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*sender, true),
        AccountMeta::new(*sender_ata, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(config_pda_ata, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    for signer_pubkey in signer_pubkeys {
        accounts.push(AccountMeta::new_readonly(*signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to collect SPL fees.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn collect_spl_fees_instruction(
    operator: &Pubkey,
    token_program_id: &Pubkey,
    mint: &Pubkey,
    receiver: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::SplToken(
        PayWithSplToken::CollectFees { amount, decimals },
    ))?;
    let (config_pda, _bump) = crate::get_config_pda();
    let config_pda_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &config_pda,
        mint,
        token_program_id,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(config_pda_ata, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to refund previously collected SPL fees.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn refund_spl_fees_instruction(
    operator: &Pubkey,
    token_program_id: &Pubkey,
    mint: &Pubkey,
    receiver: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::SplToken(PayWithSplToken::Refund {
        decimals,
        tx_hash,
        log_index,
        fees,
    }))?;
    let (config_pda, _bump) = crate::get_config_pda();
    let config_pda_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &config_pda,
        mint,
        token_program_id,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(config_pda_ata, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

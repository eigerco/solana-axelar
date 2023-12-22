//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Instructions supported by the TokenManager program.
/// Represents the different types of instructions that can be performed.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum TokenManagerInstruction {
    /// Used to set up the initial state of the program.
    /// Accounts expected by this instruction:
    ///   0. `[writeable,signer]` Funding account, pays for the account creation
    ///   1. `[writable]` The new TokenManager PDA account that needs to be
    ///      created
    ///   2. `[writable]` The new TokenManager Flow Data PDA account that needs
    ///      to be created
    ///   3. `[]` Operator Group PDA account that represents operators
    ///   4. `[]` Operator PDA account that's a part of the operator group
    ///   5. `[signer]` User account that is a part of the operator group
    ///   6. `[]` Operator Group PDA account that represents flow limiters
    ///   7. `[]` Flow limiter PDA account that's a part of the flow limiter
    ///      group
    ///   8. `[signer]` User account that is a part of the flow limiter group
    ///   9. `[]` Service program PDA account
    ///   10. `[]` The system program
    Setup(Setup),
    /// Used to set the flow limit of the account.
    /// Accounts expected by this instruction:
    ///  0. `[writeable]` The TokenManager PDA account
    ///  1. `[]` Operator Group PDA account that represents flow limiters
    ///  2. `[]` Operator PDA account that represents flow limiter
    ///  3. `[signer]` The flow limiter account
    ///  4. `[]` Operator Group PDA account that represents operators limiters
    ///  5. `[]` Service program PDA account
    SetFlowLimit {
        /// The new flow limit.
        amount: u64,
    },
    /// Used to add tokens to the flow in or out of the account.
    /// Accounts expected by this instruction:
    /// 0. `[writeable,signer]` Funding account, pays for the account creation
    /// 0. `[writeable]` The TokenManager PDA account
    /// 1. `[writable]` The new TokenManager Flow Data PDA account that needs to
    ///    be created
    /// 1. `[]` Operator Group PDA account that represents operators
    /// 2. `[]` Operator PDA account that represents operator
    /// 3. `[signer]` The operator account
    /// 4. `[]` Operator Group PDA account that represents flow limiters
    /// 5. `[]` Service program PDA account
    ///   10. `[]` The system program
    AddFlowDirection(FlowToAdd),
}

/// Setup instruction data
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Setup {
    /// The ID of the operator group.
    pub operator_group_id: String,
    /// The ID of the flow limiter group.
    pub flow_limiter_group_id: String,
    /// The initial amount of tokens that have flowed into the account.
    pub flow_limit: u64,
}

/// Flow addition instruction data
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct FlowToAdd {
    /// The amount of tokens that have flowed into the account.
    pub add_flow_in: u64,
    /// The amount of tokens that have flowed out of the account.
    pub add_flow_out: u64,
}

/// Builds a `Setup` instruction for the `TokenManager` program.
///
/// # Returns
///
/// * `Instruction` - The `Setup` instruction for the `TokenManager` program.
///
/// # Errors
///
/// Will return `ProgramError` if the instruction data cannot be serialized.
#[allow(clippy::too_many_arguments)]
pub fn build_setup_instruction(
    funder: &Pubkey,
    token_manager_pda: &Pubkey,
    operator_group_pda: &Pubkey,
    operator_pda: &Pubkey,
    operator: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    flow_limiter_pda: &Pubkey,
    flow_limiter: &Pubkey,
    service_program_pda: &Pubkey,
    setup_data: Setup,
) -> Result<Instruction, ProgramError> {
    let init_data = TokenManagerInstruction::Setup(setup_data);
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*token_manager_pda, false),
        AccountMeta::new_readonly(*operator_group_pda, false),
        AccountMeta::new_readonly(*operator_pda, false),
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new_readonly(*flow_limiter_group_pda, false),
        AccountMeta::new_readonly(*flow_limiter_pda, false),
        AccountMeta::new_readonly(*flow_limiter, true),
        AccountMeta::new_readonly(*service_program_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Builds a `SetFlowLimit` instruction for the `TokenManager` program.
pub fn build_set_flow_limit_instruction(
    token_manager_pda: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    flow_limiter_pda: &Pubkey,
    flow_limiter: &Pubkey,
    operator_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let init_data = TokenManagerInstruction::SetFlowLimit { amount };
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*token_manager_pda, false),
        AccountMeta::new_readonly(*flow_limiter_group_pda, false),
        AccountMeta::new_readonly(*flow_limiter_pda, false),
        AccountMeta::new_readonly(*flow_limiter, true),
        AccountMeta::new_readonly(*operator_group_pda, false),
        AccountMeta::new_readonly(*service_program_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Builds a `AddFlowDirection` instruction for the `TokenManager` program.
#[allow(clippy::too_many_arguments)]
pub fn build_add_flow_instruction(
    funder: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_flow_pda: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    flow_limiter_pda: &Pubkey,
    flow_limiter: &Pubkey,
    operator_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
    flow_direction: FlowToAdd,
) -> Result<Instruction, ProgramError> {
    let init_data = TokenManagerInstruction::AddFlowDirection(flow_direction);
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new_readonly(*token_manager_pda, false),
        AccountMeta::new(*token_manager_flow_pda, false),
        AccountMeta::new_readonly(*flow_limiter_group_pda, false),
        AccountMeta::new_readonly(*flow_limiter_pda, false),
        AccountMeta::new_readonly(*flow_limiter, true),
        AccountMeta::new_readonly(*operator_group_pda, false),
        AccountMeta::new_readonly(*service_program_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
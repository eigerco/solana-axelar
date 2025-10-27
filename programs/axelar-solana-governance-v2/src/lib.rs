#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod instructions;
pub use instructions::*;

pub mod state;
pub use state::*;

pub mod errors;
pub use errors::*;

pub mod events;
pub use events::*;

pub mod payload_conversions;

declare_id!("8sWy2bidXuwtZHfpJ2Ko5AiCsGQyFMf8MKwazB16wmJV");

/// Seed prefixes for different PDAs initialized by the Governance program.
pub mod seed_prefixes {
    use crate::state;

    /// The main config for the governance
    pub const GOVERNANCE_CONFIG: &[u8] = state::GovernanceConfig::SEED_PREFIX;
    /// The seed that determines a proposal PDA
    pub const PROPOSAL_PDA: &[u8] = state::ExecutableProposal::SEED_PREFIX;
    /// The seed that derives a PDA which holds a status that
    /// signals an operator can operate a proposal (like executing it
    /// regardless of the ETA).
    pub const OPERATOR_MANAGED_PROPOSAL: &[u8] = state::OperatorProposal::SEED_PREFIX;
}

#[program]
pub mod axelar_solana_governance_v2 {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        params: GovernanceConfigInit,
    ) -> Result<()> {
        instructions::initialize_config_handler(ctx, params)
    }

    pub fn update_config(ctx: Context<UpdateConfig>, params: GovernanceConfigUpdate) -> Result<()> {
        instructions::update_config_handler(ctx, params)
    }

    #[instruction(discriminator = axelar_solana_gateway_v2::executable::EXECUTE_IX_DISC)]
    pub fn process_gmp(
        ctx: Context<ProcessGmp>,
        message: axelar_solana_gateway_v2::Message,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::process_gmp_handler(ctx, message, payload)
    }

    pub fn schedule_timelock_proposal(
        ctx: Context<ScheduleTimelockProposal>,
        proposal_hash: [u8; 32],
        eta: u64,
        native_value: Vec<u8>,
        target: Vec<u8>,
        call_data: Vec<u8>,
    ) -> Result<()> {
        instructions::schedule_timelock_proposal_handler(
            ctx,
            proposal_hash,
            eta,
            native_value,
            target,
            call_data,
        )
    }

    pub fn cancel_timelock_proposal(
        ctx: Context<CancelTimelockProposal>,
        proposal_hash: [u8; 32],
        eta: u64,
        native_value: Vec<u8>,
        target: Vec<u8>,
        call_data: Vec<u8>,
    ) -> Result<()> {
        instructions::cancel_timelock_proposal_handler(
            ctx,
            proposal_hash,
            eta,
            native_value,
            target,
            call_data,
        )
    }

    pub fn approve_operator_proposal(
        ctx: Context<ApproveOperatorProposal>,
        proposal_hash: [u8; 32],
        native_value: Vec<u8>,
        target: Vec<u8>,
        call_data: Vec<u8>,
    ) -> Result<()> {
        instructions::approve_operator_proposal_handler(
            ctx,
            proposal_hash,
            native_value,
            target,
            call_data,
        )
    }

    pub fn cancel_operator_proposal(
        ctx: Context<CancelOperatorProposal>,
        proposal_hash: [u8; 32],
        native_value: Vec<u8>,
        target: Vec<u8>,
        call_data: Vec<u8>,
    ) -> Result<()> {
        instructions::cancel_operator_proposal_handler(
            ctx,
            proposal_hash,
            native_value,
            target,
            call_data,
        )
    }

    pub fn execute_timelock_proposal(
        ctx: Context<ExecuteProposal>,
        execute_proposal_data: ExecuteProposalData,
    ) -> Result<()> {
        instructions::execute_proposal_handler(ctx, execute_proposal_data)
    }

    pub fn execute_operator_proposal(
        ctx: Context<ExecuteOperatorProposal>,
        execute_proposal_data: ExecuteProposalData,
    ) -> Result<()> {
        instructions::execute_operator_proposal_handler(ctx, execute_proposal_data)
    }

    pub fn transfer_operatorship(
        ctx: Context<TransferOperatorship>,
        new_operator: [u8; 32],
    ) -> Result<()> {
        instructions::transfer_operatorship_handler(ctx, new_operator)
    }

    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>, amount: u64) -> Result<()> {
        instructions::withdraw_tokens_handler(ctx, amount)
    }
}

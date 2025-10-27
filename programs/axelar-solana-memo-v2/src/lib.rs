use anchor_lang::prelude::*;

pub mod instructions;
pub use instructions::*;

pub mod state;
pub use state::*;

use axelar_solana_gateway_v2::executable::{ExecutablePayloadEncodingScheme, Message};

declare_id!("8VRxuTLvEWsUcGsA299QQdUPaFuYkV6qkHDC5gtqt3Zc");

#[program]
pub mod memo {
    use super::*;

    /// Send a memo message cross-chain via Axelar
    pub fn send_memo(
        ctx: Context<SendMemo>,
        destination_chain: String,
        destination_address: String,
        memo: String,
    ) -> Result<()> {
        instructions::send_memo_handler(ctx, destination_chain, destination_address, memo)
    }

    pub fn init(ctx: Context<Init>) -> Result<()> {
        instructions::init_handler(ctx)
    }

    pub fn execute(
        ctx: Context<Execute>,
        message: Message,
        payload: Vec<u8>,
        encoding_scheme: ExecutablePayloadEncodingScheme,
    ) -> Result<()> {
        instructions::execute_handler(ctx, message, payload, encoding_scheme)
    }

    pub fn emit_memo(ctx: Context<EmitMemo>, message: String) -> Result<()> {
        instructions::emit_memo_handler(ctx, message)
    }
}

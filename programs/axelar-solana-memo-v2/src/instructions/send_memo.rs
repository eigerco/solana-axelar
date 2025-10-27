use anchor_lang::prelude::*;
use axelar_solana_gateway_v2::{
    cpi::accounts::CallContract, program::AxelarSolanaGatewayV2,
    seed_prefixes::CALL_CONTRACT_SIGNING_SEED, GatewayConfig,
};

#[derive(Accounts)]
pub struct SendMemo<'info> {
    /// Reference to our program
    pub memo_program: Program<'info, crate::program::Memo>,

    /// Our standardized PDA for calling the gateway
    #[account(
        seeds = [CALL_CONTRACT_SIGNING_SEED],
        bump,
    )]
    pub signing_pda: AccountInfo<'info>,

    /// The gateway configuration PDA
    #[account(
        seeds = [GatewayConfig::SEED_PREFIX],
        bump = gateway_root_pda.load()?.bump,
        seeds::program = gateway_program.key()
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,

    /// Event authority - derived from gateway program
    #[account(
        seeds = [b"__event_authority"],
        bump,
        seeds::program = axelar_solana_gateway_v2::ID,
    )]
    pub gateway_event_authority: SystemAccount<'info>,

    /// Reference to the axelar gateway program
    pub gateway_program: Program<'info, AxelarSolanaGatewayV2>,
}

pub fn send_memo_handler(
    ctx: Context<SendMemo>,
    destination_chain: String,
    destination_address: String,
    memo: String,
) -> Result<()> {
    msg!(
        "Sending memo: '{}' to chain: {} at contract: {}",
        memo,
        destination_chain,
        destination_address
    );

    let payload = memo.as_bytes().to_vec();
    let bump = ctx.bumps.signing_pda;

    let signer_seeds = &[CALL_CONTRACT_SIGNING_SEED, &[bump]];
    let signer_seeds = &[&signer_seeds[..]];

    let cpi_accounts = CallContract {
        caller: ctx.accounts.memo_program.to_account_info(),
        signing_pda: Some(ctx.accounts.signing_pda.to_account_info()),
        gateway_root_pda: ctx.accounts.gateway_root_pda.to_account_info(),
        // For event_cpi
        event_authority: ctx.accounts.gateway_event_authority.to_account_info(),
        program: ctx.accounts.gateway_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.gateway_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    axelar_solana_gateway_v2::cpi::call_contract(
        cpi_ctx,
        destination_chain,
        destination_address,
        payload,
        bump,
    )?;

    msg!("Memo sent successfully!");
    Ok(())
}

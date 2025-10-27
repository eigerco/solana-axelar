use crate::{
    verification_session::SignatureVerificationSessionData, GatewayConfig, GatewayError,
    VerifierSetTracker,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(merkle_root: [u8; 32])]
pub struct InitializePayloadVerificationSession<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [GatewayConfig::SEED_PREFIX],
        bump = gateway_root_pda.load()?.bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,

    #[account(
        init,
        payer = payer,
        space = SignatureVerificationSessionData::DISCRIMINATOR.len() + std::mem::size_of::<SignatureVerificationSessionData>(),
        seeds = [
            SignatureVerificationSessionData::SEED_PREFIX,
            merkle_root.as_ref(),
            verifier_set_tracker_pda.load()?.verifier_set_hash.as_ref(),
        ],
        bump
    )]
    pub verification_session_account: AccountLoader<'info, SignatureVerificationSessionData>,

    #[account(
        seeds = [VerifierSetTracker::SEED_PREFIX, verifier_set_tracker_pda.load()?.verifier_set_hash.as_ref()],
        bump,
        // Validate that the verifier set isn't expired
        constraint = gateway_root_pda.load()?.assert_valid_epoch(verifier_set_tracker_pda.load()?.epoch).is_ok()
            @ GatewayError::VerifierSetTooOld,
    )]
    pub verifier_set_tracker_pda: AccountLoader<'info, VerifierSetTracker>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_payload_verification_session_handler(
    ctx: Context<InitializePayloadVerificationSession>,
    _merkle_root: [u8; 32],
) -> Result<()> {
    let verification_session_account =
        &mut ctx.accounts.verification_session_account.load_init()?;

    let signing_verifier_set_hash = ctx
        .accounts
        .verifier_set_tracker_pda
        .load()?
        .verifier_set_hash;

    verification_session_account.bump = ctx.bumps.verification_session_account;
    verification_session_account
        .signature_verification
        .signing_verifier_set_hash = signing_verifier_set_hash;

    Ok(())
}

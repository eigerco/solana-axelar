use crate::{
    u256::U256, GatewayConfig, GatewayError, SignatureVerificationSessionData,
    VerifierSetRotatedEvent, VerifierSetTracker,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

#[derive(Accounts)]
#[event_cpi]
#[instruction(new_verifier_set_merkle_root: [u8; 32])]
pub struct RotateSigners<'info> {
    #[account(
        mut,
        seeds = [GatewayConfig::SEED_PREFIX],
        bump = gateway_root_pda.load()?.bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,

    #[account(
        seeds = [
            SignatureVerificationSessionData::SEED_PREFIX,
            // New verifier set merkle root is used directly as the payload hash.
            &new_verifier_set_merkle_root,
            verification_session_account.load()?.signature_verification
            .signing_verifier_set_hash.as_ref(),
        ],
        bump = verification_session_account.load()?.bump,
        // Check: signature session is complete/valid
        constraint = verification_session_account.load()?.is_valid()
            @ GatewayError::SigningSessionNotValid,
    )]
    pub verification_session_account: AccountLoader<'info, SignatureVerificationSessionData>,

    #[account(
        seeds = [
            VerifierSetTracker::SEED_PREFIX,
            verification_session_account.load()?
                .signature_verification.signing_verifier_set_hash.as_slice()
        ],
        bump = verifier_set_tracker_pda.load()?.bump,
        // Check: we got the expected verifier hash
        constraint = verifier_set_tracker_pda.load()?.verifier_set_hash == verification_session_account.load()?.signature_verification
            .signing_verifier_set_hash @ GatewayError::InvalidVerifierSetTrackerProvided,
        // Check: we aren't rotating to an already existing set
        constraint = verifier_set_tracker_pda.load()?.verifier_set_hash != new_verifier_set_merkle_root
            @ GatewayError::DuplicateVerifierSetRotation,
    )]
    pub verifier_set_tracker_pda: AccountLoader<'info, VerifierSetTracker>,

    #[account(
        init,
        payer = payer,
        space = VerifierSetTracker::DISCRIMINATOR.len() + std::mem::size_of::<VerifierSetTracker>(),
        seeds = [
            VerifierSetTracker::SEED_PREFIX,
            new_verifier_set_merkle_root.as_ref()
        ],
        bump
    )]
    pub new_verifier_set_tracker: AccountLoader<'info, VerifierSetTracker>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub operator: Option<Signer<'info>>,
}

pub fn rotate_signers_handler(
    ctx: Context<RotateSigners>,
    new_verifier_set_merkle_root: [u8; 32],
) -> Result<()> {
    let gateway_root_pda = &mut ctx.accounts.gateway_root_pda.load_mut()?;
    let verifier_set_tracker_pda = &ctx.accounts.verifier_set_tracker_pda.load()?;

    // Check current verifier set isn't expired
    let epoch = verifier_set_tracker_pda.epoch;
    gateway_root_pda.assert_valid_epoch(epoch)?;

    // we always enforce the delay unless the operator has been provided and
    // its also the Gateway operator
    // reference: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L98-L101
    let operator = ctx.accounts.operator.clone();

    let enforce_rotation_delay = operator.is_none_or(|operator| {
        let operator_matches = *operator.key == gateway_root_pda.operator;
        let operator_is_signer = operator.is_signer;
        // if the operator matches and is also the signer - disable rotation delay
        !(operator_matches && operator_is_signer)
    });

    let is_latest = gateway_root_pda.current_epoch == verifier_set_tracker_pda.epoch;

    // Check: proof is signed by latest verifiers
    if enforce_rotation_delay && !is_latest {
        return err!(GatewayError::ProofNotSignedByLatestVerifierSet);
    }

    let current_time: u64 = solana_program::clock::Clock::get()?
        .unix_timestamp
        .try_into()
        .map_err(|_err| {
            msg!("received negative timestamp");
            ProgramError::ArithmeticOverflow
        })?;

    // Check: enough time has passed since last rotation (if enforced)
    if enforce_rotation_delay && !enough_time_till_next_rotation(current_time, gateway_root_pda) {
        return err!(GatewayError::RotationCooldownNotDone);
    }

    // Update Gateway config:

    // Update the last rotation timestamp
    gateway_root_pda.last_rotation_timestamp = current_time;
    // Increment the current epoch
    gateway_root_pda.current_epoch = gateway_root_pda
        .current_epoch
        .checked_add(U256::ONE)
        .ok_or(GatewayError::EpochCalculationOverflow)?;

    // Initialize the new verifier set tracker
    let new_verifier_set_tracker = &mut ctx.accounts.new_verifier_set_tracker.load_init()?;
    new_verifier_set_tracker.bump = ctx.bumps.new_verifier_set_tracker;
    new_verifier_set_tracker.epoch = gateway_root_pda.current_epoch;
    new_verifier_set_tracker.verifier_set_hash = new_verifier_set_merkle_root;

    // Emit event
    emit_cpi!(VerifierSetRotatedEvent {
        verifier_set_hash: new_verifier_set_merkle_root,
        epoch: new_verifier_set_tracker.epoch,
    });

    Ok(())
}

fn enough_time_till_next_rotation(current_time: u64, config: &GatewayConfig) -> bool {
    let secs_since_last_rotation = current_time
        .checked_sub(config.last_rotation_timestamp)
        .expect(
            "Current time minus rotate signers last successful operation time should not underflow",
        );
    secs_since_last_rotation >= config.minimum_rotation_delay
}

use crate::seed_prefixes::{CALL_CONTRACT_SIGNING_SEED, GATEWAY_SEED};
use crate::{CallContractEvent, GatewayConfig, GatewayError};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;

#[derive(Accounts)]
#[event_cpi]
pub struct CallContract<'info> {
    /// The program that wants to call us - can be a direct signer or program
    /// CHECK: We validate the caller using is_signer flag and signing PDA verification
    pub caller: UncheckedAccount<'info>,

    /// The standardized PDA that must sign - derived from the calling program
    pub signing_pda: Option<Signer<'info>>,

    /// The gateway configuration PDA (read-only)
    #[account(
        seeds = [GATEWAY_SEED],
        bump = gateway_root_pda.load()?.bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,
}

pub fn call_contract_handler(
    ctx: Context<CallContract>,
    destination_chain: String,
    destination_contract_address: String,
    payload: Vec<u8>,
    signing_pda_bump: u8,
) -> Result<()> {
    let caller = &ctx.accounts.caller;
    let signing_pda = &ctx.accounts.signing_pda;

    if caller.is_signer {
        // Direct signer, so not a program, continue
    } else {
        // Case of a program, validate and use signing PDA
        let expected_signing_pda = Pubkey::create_program_address(
            &[CALL_CONTRACT_SIGNING_SEED, &[signing_pda_bump]],
            caller.key,
        )
        .map_err(|_| {
            msg!("Invalid call: sender must be a direct signer or a valid signing PDA must be provided");
            GatewayError::InvalidSigningPDABump
        })?;

        let pda = signing_pda.as_ref().ok_or_else(|| {
            msg!("Signing PDA must be provided when sender is a program");
            GatewayError::InvalidSigningPDA
        })?;

        require_keys_eq!(
            *pda.key,
            expected_signing_pda,
            GatewayError::InvalidSigningPDA
        );
    };

    // A valid signing PDA was provided and it's a signer, continue

    let payload_hash = keccak::hash(&payload);

    emit_cpi!(CallContractEvent {
        sender: caller.key(),
        payload_hash: payload_hash.to_bytes(),
        destination_chain,
        destination_contract_address,
        payload,
    });

    Ok(())
}

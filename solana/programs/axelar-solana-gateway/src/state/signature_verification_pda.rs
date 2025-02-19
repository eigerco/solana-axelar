//! Module for the signature verification session PDA data layout type.

use bytemuck::{Pod, Zeroable};
use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use core::mem::size_of;

use crate::{error::GatewayError, seed_prefixes};

use super::signature_verification::SignatureVerification;

/// The data layout of a signature verification PDA
///
/// This struct data layout should match the exact account data bytes.
///
/// Ideally, the payload merkle root should be a part of its seeds.
#[repr(C)]
#[allow(clippy::partial_pub_fields)]
#[derive(Zeroable, Pod, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct SignatureVerificationSessionData {
    /// Signature verification session
    pub signature_verification: SignatureVerification,
    /// Seed bump for this account's PDA
    pub bump: u8,
    /// Padding for memory alignment.
    _pad: [u8; 15],
}

impl BytemuckedPda for SignatureVerificationSessionData {}

impl SignatureVerificationSessionData {
    pub(crate) fn populate<'a>(
        pda: &AccountInfo<'a>, 
        gateway_root_pda: &AccountInfo<'a>, 
        payer: &AccountInfo<'a>, 
        system_program: &AccountInfo<'a>, 
        program_id: &Pubkey, 
        merkle_root:[u8; 32]
    )-> Result<(), ProgramError>{
        // Check: Verification PDA can be derived from provided seeds.
        // using canonical bump for the session account
        let (verification_session_pda, bump) =
            crate::get_signature_verification_pda(gateway_root_pda.key, &merkle_root);
        if verification_session_pda != *pda.key {
            return Err(GatewayError::InvalidVerificationSessionPDA.into());
        }

        // Check: the verification session account has not been initialised already
        pda
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::VerificationSessionPDAInitialised)?;

        // Use the same seeds as `[crate::get_signature_verification_pda]`, plus the
        // bump seed.
        let signers_seeds = &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            gateway_root_pda.key.as_ref(),
            &merkle_root,
            &[bump],
        ];

        // Prepare the `create_account` instruction
        program_utils::init_pda_raw(
            payer,
            pda,
            program_id,
            system_program,
            size_of::<SignatureVerificationSessionData>()
                .try_into()
                .map_err(|_err| {
                    solana_program::msg!("Unexpected u64 overflow in struct size");
                    ProgramError::ArithmeticOverflow
                })?,
            signers_seeds,
        )?;
        let mut data = pda.try_borrow_mut_data()?;
        let session = SignatureVerificationSessionData::read_mut(&mut data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        session.bump = bump;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::*;

    #[test]
    fn test_initialization() {
        let buffer = [0_u8; size_of::<SignatureVerificationSessionData>()];
        let from_pod: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        let default = &SignatureVerificationSessionData::default();
        assert_eq!(from_pod, default);
        assert_eq!(from_pod.signature_verification.accumulated_threshold, 0);
        assert_eq!(from_pod.signature_verification.signature_slots, [0_u8; 32]);
        assert!(!from_pod.signature_verification.is_valid());
    }

    #[test]
    fn test_serialization() {
        let mut buffer: [u8; size_of::<SignatureVerificationSessionData>()] =
            [42; size_of::<SignatureVerificationSessionData>()];

        let original_state;

        let updated_state = {
            let deserialized: &mut SignatureVerificationSessionData =
                bytemuck::cast_mut(&mut buffer);
            original_state = *deserialized;
            let (new_threshold, _) = deserialized
                .signature_verification
                .accumulated_threshold
                .overflowing_add(1);
            deserialized.signature_verification.accumulated_threshold = new_threshold;
            *deserialized
        };
        assert_ne!(updated_state, original_state); // confidence check

        let deserialized: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        assert_eq!(&updated_state, deserialized);
    }
}

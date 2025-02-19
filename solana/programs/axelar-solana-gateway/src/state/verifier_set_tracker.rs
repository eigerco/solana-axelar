//! Module for the `VerifierSetTracker` account type.

use axelar_message_primitives::U256;
use bytemuck::{Pod, Zeroable};
use program_utils::{BytemuckedPda, ValidPDA};
pub use core::mem::size_of;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{assert_valid_verifier_set_tracker_pda, error::GatewayError, get_verifier_set_tracker_pda, seed_prefixes};

/// Ever-incrementing counter for keeping track of the sequence of signer sets
pub type Epoch = U256;
/// Verifier set hash
pub type VerifierSetHash = [u8; 32];

/// PDA that keeps track of core information about the verifier set.
/// We keep the track of the hash + epoch (sequential order of which verifier
/// set this is)
#[repr(C)]
#[allow(clippy::partial_pub_fields)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq)]
pub struct VerifierSetTracker {
    /// The canonical bump for this account.
    pub bump: u8,
    /// Padding for the bump
    _padding: [u8; 7],
    /// The epoch associated with this verifier set
    pub epoch: Epoch,
    /// The verifier set hash
    pub verifier_set_hash: VerifierSetHash,
}

impl VerifierSetTracker {
    /// Create a new [`VerifierSetTracker`].
    #[must_use]
    pub const fn new(bump: u8, epoch: Epoch, verifier_set_hash: VerifierSetHash) -> Self {
        Self {
            bump,
            _padding: [0; 7],
            epoch,
            verifier_set_hash,
        }
    }

    pub(crate) fn create<'a>(
        program_id: &Pubkey,
        payer: &AccountInfo<'a>,
        verifier_set_pda: &AccountInfo<'a>,
        system_account: &AccountInfo<'a>,
        epoch: U256,
        verifier_set_hash: &VerifierSetHash,
    )-> Result<(), ProgramError> {

        let (_, pda_bump) = get_verifier_set_tracker_pda(*verifier_set_hash);
        
        // Check: new new verifier set PDA must be uninitialised
        verifier_set_pda
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::VerifierSetTrackerAlreadyInitialised)?;

        // Initialize the tracker account
        program_utils::init_pda_raw(
            payer,
            verifier_set_pda,
            program_id,
            system_account,
            size_of::<VerifierSetTracker>().try_into()
                .expect("unexpected u64 overflow in struct size"),
            &[
                seed_prefixes::VERIFIER_SET_TRACKER_SEED,
                verifier_set_hash.as_slice(),
                &[pda_bump],
            ],
        )?;

        // store account data
        let mut data = verifier_set_pda.try_borrow_mut_data()?;
        let tracker = VerifierSetTracker::read_mut(&mut data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        *tracker = VerifierSetTracker::new(pda_bump, epoch, *verifier_set_hash);

        // check that everything has been derived correctly
        assert_valid_verifier_set_tracker_pda(tracker, verifier_set_pda.key)?;

        Ok(())
    }


    pub(crate) fn get<'a>(verifier_set_tracker_account: &AccountInfo<'a>, program_id: &Pubkey) -> Result<(Epoch, VerifierSetHash), ProgramError> {
        verifier_set_tracker_account.check_initialized_pda_without_deserialization(program_id)?;
        let data= verifier_set_tracker_account.try_borrow_data()?;
        let verifier_set_tracker =
            VerifierSetTracker::read(&data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_verifier_set_tracker_pda(
            verifier_set_tracker,
            verifier_set_tracker_account.key,
        )?;
        Ok((verifier_set_tracker.epoch, verifier_set_tracker.verifier_set_hash))
    }
}

impl BytemuckedPda for VerifierSetTracker {}

impl core::fmt::Debug for VerifierSetTracker {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt.debug_struct("VerifierSetTracker")
            .field("bump", &self.bump)
            .field("epoch", &self.epoch)
            .field("verifier_set_hash", &hex::encode(self.verifier_set_hash))
            .finish_non_exhaustive()
    }
}

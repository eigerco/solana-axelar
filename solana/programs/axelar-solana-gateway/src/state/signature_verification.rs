//! Multi-step signature verification.

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::SigningVerifierSetInfo;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_encoding::types::verifier_set::VerifierSetLeaf;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView;
use bytemuck::{Pod, Zeroable};

use super::verifier_set_tracker::VerifierSetHash;
use super::BytemuckedPda;

/// Controls the signature verification session for a given payload.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Default, Copy, PartialEq, Eq, Debug)]
pub struct SignatureVerification {
    /// Accumulated signer threshold required to validate the payload.
    ///
    /// Is incremented on each successful verification.
    ///
    /// Set to [`u128::MAX`] once the accumulated threshold is greater than or
    /// equal the current verifier set threshold.
    pub accumulated_threshold: u128,

    /// A bit field used to track which signatures have been verified.
    ///
    /// Initially, all bits are set to zero. When a signature is verified, its
    /// corresponding bit is flipped to one. This prevents the same signature
    /// from being verified more than once, avoiding deliberate attempts to
    /// decrement the remaining threshold.
    ///
    /// Currently supports 256 slots. If the signer set maximum size needs to be
    /// increased in the future, this value must change to make roof for
    /// them.
    pub signature_slots: [u8; 32],

    /// Upon the first successful signature validation, we set the hash of the
    /// signing verifier set.
    /// This data is later used when rotating signers to figure out which
    /// verifier set was the one that actually .
    pub signing_verifier_set_hash: VerifierSetHash,
}

impl BytemuckedPda for SignatureVerification {}

/// Errors that can happen during a signature verification session.
#[derive(Debug, thiserror::Error)]
pub enum SignatureVerificationError {
    /// Used when a signature index is too high.
    #[error("Slot #{0} is out of bounds")]
    SlotIsOutOfBounds(usize),

    /// Used when someone tries to verify a signature that has already been
    /// verified before.
    #[error("Slot #{0} has been previously verified")]
    SlotAlreadyVerified(usize),

    /// Used when the Merkle inclusion proof fails to verify against the given
    /// root.
    #[error("Signer is not a member of the active verifier set")]
    InvalidMerkleProof,

    /// Used when the internal digital signature verification fails.
    #[error("Digital signature verification failed")]
    InvalidDigitalSignature,
}

impl SignatureVerification {
    /// Returns `true` if a sufficient number of signatures have been verified.
    pub fn is_valid(&self) -> bool {
        self.accumulated_threshold == u128::MAX
    }

    /// Fully process a submitted signature.
    pub fn process_signature(
        &mut self,
        verifier_info: SigningVerifierSetInfo,
        verifier_set_merkle_root: &[u8; 32],
        payload_merkle_root: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        let merkle_proof =
            rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&verifier_info.merkle_proof)
                .unwrap();
        // Check: Slot is already verified
        self.check_slot_is_done(&verifier_info.leaf)?;

        // Check: Merkle proof
        Self::verify_merkle_proof(verifier_info.leaf, &merkle_proof, verifier_set_merkle_root)?;

        // Check: Digital signature
        Self::verify_digital_signature(
            &verifier_info.leaf.signer_pubkey,
            payload_merkle_root,
            &verifier_info.signature,
        )?;

        // Update state
        self.accumulate_threshold(&verifier_info.leaf);
        self.mark_slot_done(&verifier_info.leaf)?;
        if self.signing_verifier_set_hash == [0; 32] {
            self.signing_verifier_set_hash = *verifier_set_merkle_root;
        } else if &self.signing_verifier_set_hash != verifier_set_merkle_root {
            return Err(SignatureVerificationError::InvalidDigitalSignature);
        }

        Ok(())
    }

    #[inline]
    fn check_slot_is_done(
        &self,
        signature_node: &VerifierSetLeaf,
    ) -> Result<(), SignatureVerificationError> {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        let position = signature_node.position as usize;
        let Some(slot) = signature_slots.get(position) else {
            // Index is out of bounds.
            return Err(SignatureVerificationError::SlotIsOutOfBounds(position));
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(SignatureVerificationError::SlotAlreadyVerified(position));
        }
        Ok(())
    }

    #[inline]
    fn verify_merkle_proof(
        signature_node: VerifierSetLeaf,
        merkle_proof: &rs_merkle::MerkleProof<SolanaSyscallHasher>,
        verifier_set_merkle_root: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        let leaf_hash = signature_node.hash::<SolanaSyscallHasher>();

        if merkle_proof.verify(
            *verifier_set_merkle_root,
            &[signature_node.position as usize],
            &[leaf_hash],
            signature_node.set_size as usize,
        ) {
            Ok(())
        } else {
            Err(SignatureVerificationError::InvalidMerkleProof)
        }
    }

    #[inline]
    fn verify_digital_signature(
        public_key: &PublicKey,
        message: &[u8; 32],
        signature: &Signature,
    ) -> Result<(), SignatureVerificationError> {
        let is_valid = match (signature, public_key) {
            (Signature::EcdsaRecoverable(signature), PublicKey::Secp256k1(pubkey)) => {
                verify_ecdsa_signature(pubkey, signature, message)
            }
            (Signature::Ed25519(_signature), PublicKey::Ed25519(_pubkey)) => {
                unimplemented!()
            }
            _ => {
                solana_program::msg!(
                    "Error: Invalid combination of Secp256k1 and Ed25519 signature and public key"
                );
                false
            }
        };
        if is_valid {
            return Ok(());
        }

        Err(SignatureVerificationError::InvalidDigitalSignature)
    }

    #[inline]
    fn accumulate_threshold(&mut self, signature_node: &VerifierSetLeaf) {
        self.accumulated_threshold = self
            .accumulated_threshold
            .saturating_add(signature_node.signer_weight);

        // Check threshold
        if self.accumulated_threshold >= signature_node.quorum {
            self.accumulated_threshold = u128::MAX
        }
    }
    #[inline]
    fn mark_slot_done(
        &mut self,
        signature_node: &VerifierSetLeaf,
    ) -> Result<(), SignatureVerificationError> {
        let signature_slots = self.signature_slots.view_bits_mut::<Lsb0>();
        let position = signature_node.position as usize;
        let Some(slot) = signature_slots.get_mut(position) else {
            // Index is out of bounds.
            return Err(SignatureVerificationError::SlotIsOutOfBounds(position));
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(SignatureVerificationError::SlotAlreadyVerified(position));
        }
        slot.commit(true);
        Ok(())
    }

    /// Returns the slot for a given position.
    #[inline]
    pub fn slot(&self, position: usize) -> Option<bool> {
        self.signature_slots
            .view_bits::<Lsb0>()
            .get(position)
            .as_deref()
            .copied()
    }

    /// Iterator over the signature slots.
    pub fn slots_iter(&self) -> impl Iterator<Item = bool> + '_ {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        signature_slots.into_iter().map(|slot| *slot)
    }

    /// Bit slice into the signature array
    pub fn slots(&self) -> &BitSlice<u8> {
        self.signature_slots.view_bits::<Lsb0>()
    }
}

/// Verifies an ECDSA signature against a given message and public key using the
/// secp256k1 curve.
///
/// Returns `true` if the signature is valid and corresponds to the public key
/// and message; otherwise, returns `false`.
pub fn verify_ecdsa_signature(
    pubkey: &axelar_solana_encoding::types::pubkey::Secp256k1Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::EcdsaRecoverableSignature,
    message: &[u8; 32],
) -> bool {
    // The recovery bit in the signature's bytes is placed at the end, as per the
    // 'multisig-prover' contract by Axelar. Unwrap: we know the 'signature'
    // slice exact size, and it isn't empty.
    let (signature, recovery_id) = match signature {
        [first_64 @ .., recovery_id] => (first_64, recovery_id),
    };

    // Transform from Ethereum recovery_id (27, 28) to a range accepted by
    // secp256k1_recover (0, 1, 2, 3)
    let recovery_id = if *recovery_id >= 27 {
        recovery_id - 27
    } else {
        *recovery_id
    };

    // This is results in a Solana syscall.
    let secp256k1_recover =
        solana_program::secp256k1_recover::secp256k1_recover(message, recovery_id, signature);
    let Ok(recovered_uncompressed_pubkey) = secp256k1_recover else {
        solana_program::msg!("Failed to recover ECDSA signature");
        return false;
    };

    // unwrap: provided pukey is guaranteed to be secp256k1 key
    let pubkey = libsecp256k1::PublicKey::parse_compressed(pubkey)
        .unwrap()
        .serialize();

    // we drop the const prefix byte that indicates that this is an uncompressed
    // pubkey
    let full_pubkey = match pubkey {
        [_tag, pubkey @ ..] => pubkey,
    };
    recovered_uncompressed_pubkey.to_bytes() == full_pubkey
}

/// Verifies an ECDSA signature against a given message and public key using the
/// secp256k1 curve.
///
/// Returns `true` if the signature is valid and corresponds to the public key
/// and message; otherwise, returns `false`.
#[deprecated(note = "Trying to verify Ed25519 signatures on-chain will exhaust the compute budget")]
pub fn verify_eddsa_signature(
    pubkey: &axelar_solana_encoding::types::pubkey::Ed25519Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::Ed25519Signature,
    message: &[u8; 32],
) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let verifying_key = match VerifyingKey::from_bytes(pubkey) {
        Ok(verifying_key) => verifying_key,
        Err(error) => {
            solana_program::msg!("Failed to parse signer public key: {}", error);
            return false;
        }
    };
    let signature = Signature::from_bytes(signature);
    // The implementation of `verify` only returns an atomic variant
    // `InternalError::Verify` in case of verification failure, so we can safely
    // ignore the error value.
    verifying_key.verify(message, &signature).is_ok()
}

use std::iter;
use std::marker::PhantomData;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use super::{HasheableMessageVec, Message};
use crate::hasher::generic::Keccak256Hasher;
use crate::hasher::merkle_trait::Merkle;
use crate::hasher::merkle_tree::NativeHasher;
use crate::hasher::AxelarRkyv256Hasher;
#[cfg(any(test, feature = "test-fixtures", feature = "solana"))]
use crate::hasher::{merkle_tree::SolanaSyscallHasher, solana::SolanaKeccak256Hasher};
use crate::types::VerifierSet;

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Payload {
    Messages(HasheableMessageVec),
    VerifierSet(VerifierSet),
}

impl Payload {
    /// Creates a new `Payload` instance containing a vector of messages.
    pub fn new_messages(messages: Vec<Message>) -> Self {
        Self::Messages(HasheableMessageVec::new(messages))
    }

    /// Creates a new `Payload` instance containing a verifier set.
    pub fn new_verifier_set(verifier_set: VerifierSet) -> Self {
        Self::VerifierSet(verifier_set)
    }

    /// Returns the number of elements contained within the payload.
    pub fn element_count(&self) -> usize {
        match self {
            Payload::Messages(messages) => messages.len(),
            Payload::VerifierSet(_) => 1,
        }
    }

    /// Iterates over [`Payload`] and yields [`PayloadElement`] values.
    pub fn element_iterator(&self) -> impl Iterator<Item = PayloadElement> + '_ {
        let num_messages = self.element_count() as u16;
        let mut position = 0u16;
        iter::from_fn(move || {
            if position == num_messages {
                return None;
            }
            let element = match self {
                Payload::Messages(messages) => PayloadElement::Message(MessageElement {
                    message: (messages[position as usize]).clone(),
                    position,
                    num_messages,
                }),
                Payload::VerifierSet(verifier_set) => {
                    PayloadElement::VerifierSet(verifier_set.clone())
                }
            };
            position += 1;
            Some(element)
        })
    }
}

impl TryFrom<Payload> for HasheableMessageVec {
    type Error = ();
    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        match value {
            Payload::Messages(messages) => Ok(messages),
            _ => Err(()),
        }
    }
}

impl TryFrom<Payload> for VerifierSet {
    type Error = ();
    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        match value {
            Payload::VerifierSet(verifier_set) => Ok(verifier_set),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageElement {
    pub message: Message,
    pub position: u16,
    pub num_messages: u16,
}

/// A [`Payload`] element.
#[derive(Debug, Clone)]
pub enum PayloadElement {
    Message(MessageElement),
    VerifierSet(VerifierSet),
}

/// Wraps a [`PayloadElement`], is generic over the hashing context.
///
/// This type is the leaf node of a [`Payload`]'s Merkle tree.
#[derive(Debug, Clone)]
pub struct PayloadLeafNode<T> {
    pub element: PayloadElement,
    pub hasher: PhantomData<T>,
}

impl<'a, T> PayloadLeafNode<T>
where
    VerifierSet: Merkle<T>,
    T: rs_merkle::Hasher<Hash = [u8; 32]>,
{
    /// Converts this leaf node into bytes that will become the leaf nodes of a
    /// [`Payload`]'s Merkle tree.
    #[inline]
    pub fn leaf_hash<H>(&'a self) -> [u8; 32]
    where
        H: AxelarRkyv256Hasher<'a>,
    {
        match &self.element {
            PayloadElement::Message(MessageElement {
                message,
                position,
                num_messages,
            }) => {
                let mut hasher = H::default();
                hasher.hash(&[0]); // Leaf node discriminator
                hasher.hash(b"message");
                hasher.hash(bytemuck::cast_ref::<_, [u8; 2]>(position));
                hasher.hash(bytemuck::cast_ref::<_, [u8; 2]>(num_messages));
                message.hash(hasher)
            }
            PayloadElement::VerifierSet(verifier_set) => {
                // When the Payload contains a verifier set, we use the Merkle root for that
                // verifier set hash directly.
                let verifier_set_merkle_root =
                    <VerifierSet as Merkle<T>>::calculate_merkle_root(verifier_set)
                        .expect("Can't use an empty verifier set");
                let payload_element_leaf_hash =
                    H::hash_instant(&[VerifierSet::HASH_PREFIX, &verifier_set_merkle_root]);
                payload_element_leaf_hash.0
            }
        }
    }
}

#[cfg(any(test, feature = "test-fixtures", feature = "solana"))]
impl From<PayloadLeafNode<SolanaSyscallHasher>> for [u8; 32] {
    fn from(payload_leaf_node: PayloadLeafNode<SolanaSyscallHasher>) -> Self {
        payload_leaf_node.leaf_hash::<SolanaKeccak256Hasher>()
    }
}

impl From<PayloadLeafNode<NativeHasher>> for [u8; 32] {
    fn from(payload_leaf_node: PayloadLeafNode<NativeHasher>) -> Self {
        payload_leaf_node.leaf_hash::<Keccak256Hasher>()
    }
}

impl<H> Merkle<H> for Payload
where
    H: rs_merkle::Hasher<Hash = [u8; 32]>,
    PayloadLeafNode<H>: Into<[u8; 32]>,
{
    type LeafNode = PayloadLeafNode<H>;

    fn merkle_leaves(&self) -> impl Iterator<Item = PayloadLeafNode<H>> {
        self.element_iterator().map(|element| PayloadLeafNode {
            element,
            hasher: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::hasher::merkle_trait::tests::assert_merkle_inclusion_proof;
    use crate::test_fixtures::{random_payload, random_valid_verifier_set};

    /// Tests the consistency and validity of Merkle inclusion proofs across
    /// different hasher implementations.
    #[test]
    fn test_payload_merkle_inclusion_proof() {
        let payload = random_payload();

        assert_eq!(
            assert_merkle_inclusion_proof::<SolanaSyscallHasher, _>(&payload),
            assert_merkle_inclusion_proof::<NativeHasher, _>(&payload),
            "different hasher implementations should produce the same merkle root"
        )
    }

    /// Tests the Merkle root and inclusion proof relationships between a
    /// payload containing a verifier set and the verifier set itself.
    ///
    /// Specifically, it checks that the Merkle root of the verifier set serves
    /// as a valid inclusion proof for the Merkle root of the payload containing
    /// that verifier set.
    #[test]
    fn test_merkle_proofs_for_payload_with_verifier_set() {
        // Setup: get a random verifier set and wrap it into a payload
        let verifier_set = random_valid_verifier_set();
        let payload = Payload::VerifierSet(verifier_set.clone());

        // Calculate the Merkle root, leaf node hash and inclusion proof for the
        // payload. Naturally, they refer to the verifier set we started this
        // test with.
        let payload_merkle_root =
            <Payload as Merkle<NativeHasher>>::calculate_merkle_root(&payload).unwrap();

        let payload_leaf_hash: [u8; 32] =
            <Payload as Merkle<NativeHasher>>::merkle_leaves(&payload)
                .next()
                .unwrap()
                .into();

        let payload_leaf_inclusion_proof =
            <Payload as Merkle<NativeHasher>>::merkle_proofs(&payload)
                .next()
                .unwrap();

        // Calculate the merkle root for the verifier set.
        let verifier_set_merkle_root =
            <VerifierSet as Merkle<NativeHasher>>::calculate_merkle_root(&verifier_set).unwrap();

        assert_eq!(
            payload_leaf_hash,
            verifier_set_merkle_root,
            "the hash of the single leaf node in a payload containing a verifier set should match the Merkle root of that same verifier set."
        );

        assert!(payload_leaf_inclusion_proof.verify(
            payload_merkle_root,
            &[0],
            &[verifier_set_merkle_root],
            payload.element_count()
        ),
            "the Merkle root of the verifier set should be a valid inclusion proof for the payload's Merkle root in which it is included."
        )
    }
}

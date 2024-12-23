use std::error::Error;

use rkyv::bytecheck::{self, CheckBytes, StructCheckError};
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{AlignedVec, Archive, Deserialize, Serialize};

use super::ArchivedHasheableMessageVec;
use crate::hasher::AxelarRkyv256Hasher;
use crate::types::{
    ArchivedPayload, ArchivedProof, ArchivedVerifierSet, Payload, Proof, VerifierSet,
};
use crate::visitor::{ArchivedVisitor, Visitor};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct ExecuteData {
    pub payload: Payload,
    pub proof: Proof,
}

impl ExecuteData {
    pub fn new(payload: Payload, proof: Proof) -> Self {
        Self { payload, proof }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    pub fn to_bytes<const SCRATCH: usize>(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        rkyv::to_bytes::<_, SCRATCH>(self)
            .map(AlignedVec::into_vec)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

impl ArchivedExecuteData {
    pub fn proof(&self) -> &ArchivedProof {
        &self.proof
    }

    pub fn messages(&self) -> Option<&ArchivedHasheableMessageVec> {
        match &self.payload {
            ArchivedPayload::Messages(messages) => Some(messages),
            _ => None,
        }
    }

    pub fn verifier_set(&self) -> Option<&ArchivedVerifierSet> {
        match &self.payload {
            ArchivedPayload::VerifierSet(verifier_set) => Some(verifier_set),
            _ => None,
        }
    }

    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        ArchivedVisitor::visit_execute_data(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn hash_payload_for_verifier_set<'a>(
        &'a self,
        domain_separator: &'a [u8; 32],
        verifier_set: &'a VerifierSet,
        mut hasher_impl: impl AxelarRkyv256Hasher<'a>,
    ) -> [u8; 32] {
        Visitor::visit_bytes(&mut hasher_impl, domain_separator);
        Visitor::visit_verifier_set(&mut hasher_impl, verifier_set);
        ArchivedVisitor::visit_payload(&mut hasher_impl, &self.payload);
        hasher_impl.result().into()
    }

    /// Produces the same hash as [`crate::hash_payload`].
    pub fn internal_payload_hash<'a>(
        &'a self,
        domain_separator: &'a [u8; 32],
        mut hasher_impl: impl AxelarRkyv256Hasher<'a>,
    ) -> [u8; 32] {
        Visitor::visit_bytes(&mut hasher_impl, domain_separator);
        self.proof
            .drive_visitor_for_signer_set_hash(&mut hasher_impl, domain_separator);
        ArchivedVisitor::visit_payload(&mut hasher_impl, &self.payload);
        hasher_impl.result().into()
    }

    pub fn from_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<StructCheckError, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<ExecuteData>(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{random_execute_data, test_hasher_impl};

    #[test]
    fn test_serialize_deserialize_execute_data() {
        let execute_data = random_execute_data();

        let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

        assert_eq!(*archived, execute_data);
    }

    #[test]
    fn archived_and_unarchived_values_have_the_same_hash() {
        let execute_data = random_execute_data();

        let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

        let mut archived_hasher = test_hasher_impl();
        let mut unarchived_hasher = test_hasher_impl();

        Visitor::visit_execute_data(&mut unarchived_hasher, &execute_data);
        ArchivedVisitor::visit_execute_data(&mut archived_hasher, archived);

        assert_eq!(archived_hasher.result(), unarchived_hasher.result());
    }
}

use std::error::Error;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use crate::hasher::Hasher;
use crate::visitor::{ArchivedVisitor, Visitor};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct CrossChainId {
    pub(crate) chain: String,
    pub(crate) id: String,
}

impl CrossChainId {
    pub fn new(chain: String, id: String) -> Self {
        Self { chain, id }
    }
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        Visitor::visit_cc_id(&mut hasher, self);
        hasher.finalize()
    }

    pub fn chain(&self) -> &str {
        &self.chain
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl ArchivedCrossChainId {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        ArchivedVisitor::visit_cc_id(&mut hasher, self);
        hasher.finalize()
    }

    pub fn chain(&self) -> &str {
        &self.chain
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Message {
    pub(crate) cc_id: CrossChainId,
    pub(crate) source_address: String,
    pub(crate) destination_chain: String,
    pub(crate) destination_address: String,
    pub(crate) payload_hash: [u8; 32],
}

impl Message {
    pub fn new(
        cc_id: CrossChainId,
        source_address: String,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
    ) -> Self {
        Self {
            cc_id,
            source_address,
            destination_chain,
            destination_address,
            payload_hash,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        Visitor::visit_message(&mut hasher, self);
        hasher.finalize()
    }

    pub fn cc_id(&self) -> &CrossChainId {
        &self.cc_id
    }

    pub fn destination_address(&self) -> &str {
        &self.destination_address
    }

    pub fn source_address(&self) -> &str {
        &self.source_address
    }

    pub fn payload_hash(&self) -> &[u8; 32] {
        &self.payload_hash
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        rkyv::to_bytes::<_, 0>(self)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            .map(|bytes| bytes.to_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

impl ArchivedMessage {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        ArchivedVisitor::visit_message(&mut hasher, self);
        hasher.finalize()
    }

    pub fn cc_id(&self) -> &ArchivedCrossChainId {
        &self.cc_id
    }

    pub fn destination_address(&self) -> &str {
        &self.destination_address
    }

    pub fn source_address(&self) -> &str {
        &self.source_address
    }

    pub fn payload_hash(&self) -> &[u8; 32] {
        &self.payload_hash
    }

    pub fn from_archived_bytes(bytes: &[u8]) -> &Self {
        unsafe { rkyv::archived_root::<Message>(bytes) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::random_message;

    #[test]
    fn unarchived_roundtrip() {
        let message = random_message();

        let bytes = message.to_bytes().unwrap();
        let deserialized = Message::from_bytes(&bytes).unwrap();

        assert_eq!(message, deserialized);
    }
}

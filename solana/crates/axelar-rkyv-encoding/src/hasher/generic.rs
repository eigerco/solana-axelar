use std::marker::PhantomData;

use sha3::{Digest, Keccak256};

use super::{AxelarRkyv256Hasher, Hash256};
use crate::visitor::{ArchivedVisitor, Visitor};

#[derive(Clone, Default)]
pub struct Keccak256Hasher<'a> {
    hasher: Keccak256,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Visitor<'a> for Keccak256Hasher<'a> {
    fn visit_bytes(&mut self, bytes: &'a [u8]) {
        self.hasher.update(bytes)
    }

    fn tag(&mut self, bytes: &'a [u8]) {
        Visitor::visit_bytes(self, bytes)
    }
}

impl<'a> ArchivedVisitor<'a> for Keccak256Hasher<'a> {
    fn visit_bytes(&mut self, bytes: &'a [u8]) {
        self.hasher.update(bytes)
    }

    fn tag(&mut self, bytes: &'a [u8]) {
        ArchivedVisitor::visit_bytes(self, bytes)
    }
}

impl<'a> AxelarRkyv256Hasher<'a> for Keccak256Hasher<'a> {
    fn hash(&mut self, val: &'a [u8]) {
        self.hasher.update(val)
    }
    fn hashv(&mut self, vals: &'a [&[u8]]) {
        for val in vals {
            self.hash(val);
        }
    }
    fn hash_instant(vals: &[&[u8]]) -> Hash256 {
        let mut hasher = Keccak256::new();
        for val in vals {
            hasher.update(val)
        }
        Hash256(hasher.finalize().into())
    }
    fn result(self) -> Hash256 {
        Hash256(self.hasher.finalize().into())
    }
}

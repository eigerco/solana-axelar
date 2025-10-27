use anchor_lang::prelude::*;

#[event]
pub struct TrustedChainSet {
    pub chain_name: String,
}

#[event]
pub struct TrustedChainRemoved {
    pub chain_name: String,
}

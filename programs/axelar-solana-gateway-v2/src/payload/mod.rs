use anchor_lang::error_code;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::AccountMeta;

mod abi;
mod accounts;
mod borsh;
mod encoding;

pub use accounts::SolanaAccountRepr;
pub use encoding::EncodingScheme;

/// In standard Axelar flow, the accounts are concatenated at the beginning of
/// the payload message. This struct represents a Solana account in a way that
/// can be easily serialized and deserialized.
///
/// The payload is encoded in the following way:
/// - the first byte is encoding scheme, encoded as an u8.
/// - the rest of the data is encoded([account array][payload bytes]). The
///   encoding depends on the encoding scheme.
///
/// ```text
/// [u8 scheme] encoded([account array][payload bytes])
/// ```
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct AxelarMessagePayload<'payload> {
    // Using Cow because on-chain we will use a the owned version (because of the decoding),
    // but off-chain we will use the borrowed version to prevent unnecessary cloning.
    payload_without_accounts: &'payload [u8],
    solana_accounts: Vec<SolanaAccountRepr>,
    encoding_scheme: EncodingScheme,
}

impl<'payload> AxelarMessagePayload<'payload> {
    /// Create a new payload from a "payload without accounts" and a list of
    /// accounts representations.
    pub fn new<T>(
        payload_without_accounts: &'payload [u8],
        solana_accounts: &[T],
        encoding_scheme: EncodingScheme,
    ) -> Self
    where
        for<'b> &'b T: Into<SolanaAccountRepr>,
    {
        let solana_accounts = solana_accounts.iter().map(Into::into).collect();
        Self {
            payload_without_accounts,
            solana_accounts,
            encoding_scheme,
        }
    }

    /// Get the payload hash.
    ///
    /// # Errors
    /// - the payload struct cannot be encoded
    pub fn hash(&self) -> Result<[u8; 32], PayloadError> {
        let payload = self.encode()?;
        let payload_hash = Self::hash_payload(&payload);
        Ok(payload_hash)
    }

    pub fn hash_payload(payload: &[u8]) -> [u8; 32] {
        solana_program::keccak::hash(payload).to_bytes()
    }

    /// Get the payload without accounts.
    #[must_use]
    pub const fn payload_without_accounts(&self) -> &[u8] {
        self.payload_without_accounts
    }

    /// Get the solana accounts.
    #[must_use]
    pub fn account_meta(&self) -> Vec<AccountMeta> {
        self.solana_accounts
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Get an iterator over the Solana accounts
    pub fn solana_accounts(&self) -> impl Iterator<Item = &SolanaAccountRepr> {
        self.solana_accounts.iter()
    }

    /// Get the underlying encoding scheme used by the [`AxelarMessagePayload`]
    #[must_use]
    pub const fn encoding_scheme(&self) -> EncodingScheme {
        self.encoding_scheme
    }
}

#[error_code]
pub enum PayloadError {
    /// Invalid encoding scheme
    #[msg("Invalid encoding scheme")]
    InvalidEncodingScheme,

    /// Borsh serialization error
    #[msg("Borsh serialize error")]
    BorshSerializeError,

    /// Borsh deserialization error
    #[msg("Borsh deserialize error")]
    BorshDeserializeError,

    /// ABI error
    #[msg("ABI error")]
    AbiError,

    /// Internal type conversion error
    #[msg("Internal type conversion error")]
    Conversion,
}

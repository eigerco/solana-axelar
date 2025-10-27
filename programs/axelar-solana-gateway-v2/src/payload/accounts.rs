use alloy_sol_types::sol;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::instruction::AccountMeta;

//
// Accounts
//

sol! {
    /// Representation of a Solana account in a way that can be easily serialized
    /// for Payload consumption.
    ///
    /// This is the expected data type that will be used to represent Solana
    /// accounts in the serilaized payload format.
    ///
    /// Utility methods are provided to encode and decode the representation.
    #[derive(Debug, PartialEq, Eq, Copy)]
    #[repr(C)]
    struct SolanaAccountRepr {
        /// Solana Pubkey (decoded format -- raw bytes)
        bytes32 pubkey;
        /// flag to indicate if the account is signer
        bool is_signer;
        /// flag to indicate if the account is writable
        bool is_writable;
    }
}

impl PartialEq<AccountInfo<'_>> for SolanaAccountRepr {
    fn eq(&self, other: &AccountInfo<'_>) -> bool {
        self.pubkey.as_slice() == other.key.as_ref()
            && self.is_signer == other.is_signer
            && self.is_writable == other.is_writable
    }
}

// NOTE: Mostly used by tests
impl<'a> From<&'a Self> for SolanaAccountRepr {
    fn from(value: &'a Self) -> Self {
        *value
    }
}

impl<'a, 'b> From<&'b AccountInfo<'a>> for SolanaAccountRepr {
    fn from(account: &'b AccountInfo<'a>) -> Self {
        Self {
            pubkey: account.key.to_bytes().into(),
            is_signer: account.is_signer,
            is_writable: account.is_writable,
        }
    }
}

impl<'a> From<&'a AccountMeta> for SolanaAccountRepr {
    fn from(value: &'a AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<AccountMeta> for SolanaAccountRepr {
    fn from(value: AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<SolanaAccountRepr> for AccountMeta {
    fn from(value: SolanaAccountRepr) -> Self {
        let pubkey_bytes: [u8; 32] = value.pubkey.into();

        Self {
            pubkey: pubkey_bytes.into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;

    #[test]
    fn solana_account_repr_account_info_conversions() {
        for (is_singer, is_writer) in &[(true, true), (true, false), (false, true), (false, false)]
        {
            let key = Pubkey::new_unique();
            let mut lamports = 100;
            let account = AccountInfo::new(
                &key,
                *is_singer,
                *is_writer,
                &mut lamports,
                &mut [],
                &key,
                false,
                0,
            );
            let repr = SolanaAccountRepr::from(&account);
            assert_eq!(repr.is_signer, *is_singer, "Signer flag is gone!");
            assert_eq!(repr.is_writable, *is_writer, "Writable flag is gone!");
            assert_eq!(
                repr.pubkey.to_vec()[..],
                key.to_bytes()[..],
                "Pubkey does not match!"
            );
        }
    }
}

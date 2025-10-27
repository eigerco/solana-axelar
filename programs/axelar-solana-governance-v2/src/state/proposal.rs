use anchor_lang::prelude::*;
use anchor_lang::solana_program;

#[derive(Debug, InitSpace)]
#[account]
pub struct ExecutableProposal {
    /// Represent the le bytes containing unix timestamp from when the proposal
    /// can be executed.
    pub eta: u64,
    /// The bump seed for the proposal PDA.
    pub bump: u8,
    /// The bump seed for the operator managed proposal PDA.
    pub managed_bump: u8,
}

type Uint256 = [u8; 32];
type Hash = [u8; 32];

impl ExecutableProposal {
    pub const SEED_PREFIX: &'static [u8] = b"proposal";

    pub fn find_pda(proposal_hash: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::SEED_PREFIX, proposal_hash], &crate::ID)
    }

    pub fn hash_from_data(data: &ExecuteProposalData) -> Hash {
        let target = Pubkey::new_from_array(data.target_address);
        let call_data = &data.call_data;
        let native_value = &data.native_value;
        Self::calculate_hash(&target, call_data, native_value)
    }

    pub fn calculate_hash(
        target: &Pubkey,
        call_data: &ExecuteProposalCallData,
        native_value: &Uint256,
    ) -> Hash {
        let sol_accounts_ser = borsh::to_vec(&call_data.solana_accounts)
            .expect("Solana accounts serialization failed");
        let native_value_ser = borsh::to_vec(&call_data.solana_native_value_receiver_account)
            .expect("Solana native value receiver account serialization failed");
        let call_data_ser = &call_data.call_data;

        solana_program::keccak::hashv(&[
            &target.to_bytes(),
            sol_accounts_ser.as_ref(),
            native_value_ser.as_ref(),
            call_data_ser,
            native_value,
        ])
        .to_bytes()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ExecuteProposalData {
    /// The target program address for the proposal, represented as a 32-byte
    /// array. Will be later converted to a [`solana_program::pubkey::Pubkey`].
    /// [`solana_program::pubkey::Pubkey`] when executing the proposal.
    pub target_address: [u8; 32],
    /// The data required to call the target program.
    pub call_data: ExecuteProposalCallData,
    /// A 32-byte array representing the native token U256 value (lamports)
    /// associated with the proposal. This is a U256 value and should be casted
    /// to u 64
    pub native_value: [u8; 32],
}

#[derive(Debug, Eq, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ExecuteProposalCallData {
    /// The Solana accounts metadata required for the target program in the
    /// moment of the proposal execution.
    ///
    /// In case the target program requires a native token transfer, the first
    /// account should be the target account the proposal should transfer the
    /// funds to.
    pub solana_accounts: Vec<SolanaAccountMetadata>,

    /// Apart from the [`Self::solana_accounts`] metadata, and in case the
    /// proposal requires a native token transfer to the target contract, the
    /// receiver account should be set here.
    pub solana_native_value_receiver_account: Option<SolanaAccountMetadata>,

    /// The call data required to execute the target program.
    pub call_data: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SolanaAccountMetadata {
    /// The [`solana_program::pubkey::Pubkey`], converted to bytes.
    pub pubkey: [u8; 32],
    /// If this account is a signer of the transaction. See original
    /// [`solana_program::instruction::AccountMeta::is_signer`].
    pub is_signer: bool,
    /// If this account is writable. See original
    /// [`solana_program::instruction::AccountMeta::is_writable`].
    pub is_writable: bool,
}

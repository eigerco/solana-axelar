#![deny(missing_docs)]

//! Program utility functions

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use axelar_solana_encoding::types::messages::Message;
use solana_program::instruction::{AccountMeta, Instruction};

use crate::structs::{RelayerAccount, RelayerData, RelayerInstruction, RelayerTransaction};

/// The structs for relayer discovery.
pub mod structs;
/// Useful functions for testing.
pub mod test;

/// The global transaction pda seed for all executables.
pub const TRANSACTION_PDA_SEED: &[u8] = b"relayer-discovery-transaction";

/// mini helper to log from native Rust or to the program log
/// Very useful for debugging when you have to run some code on Solana and via
/// native Rust
#[macro_export]
macro_rules! log_everywhere {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);

        #[cfg(not(target_os = "solana"))]
        {
            dbg!(message);
        }

        #[cfg(target_os = "solana")]
        {
            solana_program::msg!("SOL: {}", message);
        }
    }}
}

/// A struct for keeping track of relayer discovery.
pub struct RelayerDiscovery {
    /// The message that is being relayed.
    pub message: Message,
    /// The message pda.
    pub message_pda: Pubkey,
    /// The raw payload.
    pub payload: Vec<u8>,
    /// The payload pda, if uploading is done.
    pub payload_pda: Option<Pubkey>,
    /// A list of dedicated payer accounts.
    pub payers: Vec<(Pubkey, u64)>,
}

/// An error when converting between relayer and actual accounts. 
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The message payload needs to be uploaded before further relayer discovery can be done.
    #[error("The message payload needs to be uploaded before further relayer discovery can be done")]
    NeedMessagePayload,
    /// A dedicated payer account needs to be added with the specified lamports available.
    #[error("A dedicated payer account needs to be added with the specified lamports available")]
    NeedPayer(u64),
    /// Failed to serialize message.
    #[error("Failed to serialize message")]
    FailedMessageSerialization,
    /// Failed to serialize message.
    #[error("Failed to serialize payload")]
    FailedPayloadSerialization,
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// A struct to capture the two kinds of transaction that the relayer should be able to pefrorm.
pub enum ConvertedTransaction {
    /// This is the final transaction that should be committed to the blockchain.
    Final(Vec<Instruction>),
    /// This should just be simulated, expecting another `RelayerTransaction` as the result.
    Discovery(Instruction),
}

impl RelayerDiscovery {
    /// Converts a `RelayerAccount` into an `AccountMeta` instance. Can return a `ConvertError` error if the relayer needs to upload the payload to a pda/add a payer.
    pub fn convert_account(self: &Self, account: &RelayerAccount, used_payers: &mut Vec<usize>) -> Result<AccountMeta, ConvertError> {
        match account {
            RelayerAccount::Account{ pubkey, is_writable} => if *is_writable {
                Ok(AccountMeta::new(*pubkey, false))
            } else {
                Ok(AccountMeta::new_readonly(*pubkey, false))
            }
            RelayerAccount::IncomingMessage => Ok(AccountMeta { pubkey: self.message_pda, is_signer: false, is_writable: false }),
            RelayerAccount::MessagePayload => match self.payload_pda {
                Some(payload_pda) => Ok(AccountMeta { pubkey: payload_pda, is_signer: false, is_writable: false }),
                None => Err(ConvertError::NeedMessagePayload),
            },
            RelayerAccount::Payer(lamports) => {
                let payer = self.payers.iter().enumerate().find(|(index, payer)| payer.1 == *lamports && !used_payers.contains(index));
                match payer {
                    Some((index, (pubkey, _))) => {
                        used_payers.push(index);
                        Ok(AccountMeta::new(*pubkey, true))
                    },
                    None => Err(ConvertError::NeedPayer(*lamports)),
                }
            }
        }
    }

    /// Converts a `RelayerData` struct into `Vec<u8>`. Can return `ConvertError` if serialization of `Message` fails.
    pub fn convert_data(self: &Self, data: &RelayerData) -> Result<Vec<u8>, ConvertError> {
        match data {
            RelayerData::Bytes(bytes) => {
                Ok(bytes.clone())
            },
            RelayerData::Message => {
                to_vec(&self.message).map_err(|_| ConvertError::FailedMessageSerialization)
            },
            RelayerData::Payload => {
                to_vec(&self.payload).map_err(|_| ConvertError::FailedPayloadSerialization)
            },
        }
    }

    /// Converts a whole `RelayerInstruction` to an `Instruction`. Can return a `ConvertError` in cases outlined in `convert_account` and `convert_data`.
    pub fn convert_instruction(self: &Self, instruction: &RelayerInstruction) -> Result<Instruction, ConvertError> {
        let mut used_payers = vec![];
        let accounts: Result<Vec<AccountMeta>, ConvertError> = instruction.accounts.iter().map(|account| self.convert_account(account, &mut used_payers)).collect();
        let data: Result<Vec<Vec<u8>>, ConvertError> = instruction.data.iter().map(|data| self.convert_data(data)).collect();
        //let data = instruction.data.iter().map(|data| self.convert_data(data)).collect()?;
        Ok(Instruction {
            program_id: instruction.program_id,
            accounts: accounts?,
            data: data?.concat(),
        })
    }

    /// Converts a `RelayerTransaction` into a `ConvertedTransaction` that can be used to make RPC calls.
    pub fn convert_transaction(self: &Self, transaction: &RelayerTransaction) -> Result<ConvertedTransaction, ConvertError> {
        match transaction {
            RelayerTransaction::Final(instructions) => {
                let instructions: Result<Vec<Instruction>, ConvertError> = instructions.iter().map(|instruction| self.convert_instruction(instruction)).collect();
                Ok(ConvertedTransaction::Final(instructions?))
            },
            RelayerTransaction::Discovery(instruction) => {
                Ok(ConvertedTransaction::Discovery(self.convert_instruction(instruction)?))
            },
        }
    }

    /// Add a payer to a relayer discovery.
    pub fn add_payer(
		&mut self,
		payer: Pubkey,
		amount: u64,
	) {
		self.payers.push((payer, amount));
	}

    /// Add a payload pda to the relayer discovery.
    pub fn add_payload_pda(
        &mut self,
        payload_pda: Pubkey,
    ) {
        self.payload_pda = Some(payload_pda);
    }
}

/// find the transaction pda for a given program id
pub fn find_transaction_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TRANSACTION_PDA_SEED], program_id)
}



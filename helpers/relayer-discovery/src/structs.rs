#![deny(missing_docs)]

use solana_program::pubkey::Pubkey;
use program_utils::{log_everywhere, pda::{init_pda_raw_bytes, BorshPda}};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint_deprecated::ProgramResult};
use solana_sdk::msg;


#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
/// A single piece of data to be passed by the relayer. Each of these can be converted to Vec<u8>.
pub enum RelayerData {
	/// Some raw bytes.
	Bytes(Vec<u8>),
	/// The message.
	Message,
	/// The payload, length prefixed.
	Payload,
}
#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
/// This can be used to specify an account that the relayer will pass to the executable. This can be converted to an `AccountMeta` by the relayer.
pub enum RelayerAccount {
	/// This variant specifies a specific account. This account cannot be a signer (see `Payer`` below).
	Account{
		/// The pubkey of the account.
		pubkey: Pubkey,
		/// Whether or not this account is writable.
		is_writable: bool,
	},
	/// The incoming message PDA, which contains all the message information aside from the payload. This should only be specified once per instruction.
	IncomingMessage,
	/// An account that has the payload as its data. This account if and only if it is requested by the executable. This should only be specified once per instruction.
	MessagePayload,
	/// A signer account that has the amount of lamports specified. These lamports will be subtracted from the gas for the execution of the program. 
	/// This can be specified multiple times per instruction, and multiple payer accounts, funded differently will be provided. (Do we want this?)
	Payer(u64)
}
#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
/// A relayer instruction, that the relayer can convert to an `Instruction`.
pub struct RelayerInstruction {
	/// The program_id
	pub program_id: Pubkey,
	/// The instruction accounts.
	pub accounts: Vec<RelayerAccount>,
	/// The instruction data.
	pub data: Vec<RelayerData>,
}

#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
/// A relayer transaction, that the relayer can convert to regular transaction.
pub enum RelayerTransaction {
	/// This series of instructions should be executed.
	Final(Vec<RelayerInstruction>),
	/// This instruction should be simulated to eventually get a `Final` transaction.
	Discovery(RelayerInstruction),
}

impl RelayerTransaction {
	/// Helper function that serializes the enum. There must be a better way of doing this that escapes me.
	pub fn init<'a>(
        &self,
        program_id: &Pubkey,
        system_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        into: &AccountInfo<'a>,
        signer_seeds: &[&[u8]],
    ) -> ProgramResult {
		let serialized_data = to_vec(self)?;
		match self {
			Self::Discovery(ix) => {
				msg!("{}", ix.data.len());
			}
			_ => ()
		}

        init_pda_raw_bytes(
            payer,
            into,
            program_id,
            system_account,
            &serialized_data,
            signer_seeds,
        )?;

        Ok(())
	}
}
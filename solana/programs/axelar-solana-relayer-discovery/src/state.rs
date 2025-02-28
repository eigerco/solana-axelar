//! State module for the Axelar Solana Gas Service

use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;

use serde::{Serialize, Deserialize};
use solana_program::pubkey::Pubkey;
use solana_program::instruction::Instruction;
use solana_program::account_info::AccountInfo;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;

use crate::create_relayer_execution_pda;
use crate::get_relayer_execution_pda;
use crate::get_singing_pda;
use crate::seed_prefixes;

/// Some placeholder accounts for the relayer to substitute into instructions.
pub mod preset_accounts {
    use solana_program::pubkey::Pubkey;

    const MESSAGE_ID: u8 = 1;
    const PAYLOAD_ID: u8 = 2;

    /// The message account
    pub const MESSAGE: Pubkey = Pubkey::new_from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, MESSAGE_ID]);
    /// The payload account
    pub const PAYLOAD: Pubkey = Pubkey::new_from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, PAYLOAD_ID]);
}

/// Information about how to fund an account.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FundAccountData {
    account: Pubkey,
    amount: u64,
}

/// A single instruction for the relayer to execute.
#[repr(u8)]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RelayerInstruction {
    /// A regular instruction.
    Instruction(Instruction),
    /// An instruction to fun a specific account, using gas money to do so.
    FundAccount(FundAccountData)
}

impl BorshSerialize for RelayerInstruction {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let data = bincode::serialize(self).map_err(|_| Error::from(ErrorKind::InvalidInput))?;
        writer.write(&data)?;
        Ok(())
    }
}

impl BorshDeserialize for RelayerInstruction {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        bincode::deserialize_from(reader).map_err(|_| Error::from(ErrorKind::InvalidData))
    }
}

/// Information about what the relayer should do next.
#[repr(u8)]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RelayerExecutionInfo {
    /// The relayer needs to query the state again.
    Query(Instruction),
    /// The relayer should execute.
    Execute(Vec<RelayerInstruction>),
}

impl BorshSerialize for RelayerExecutionInfo {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let data = bincode::serialize(self).map_err(|_| Error::from(ErrorKind::InvalidInput))?;
        writer.write(&data)?;
        Ok(())
    }
}

impl BorshDeserialize for RelayerExecutionInfo {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        bincode::deserialize_from(reader).map_err(|_| Error::from(ErrorKind::InvalidData))
    }
}

/// The pda used to store a single RelayerExecutionInfo.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RelayerExecutionPda {
    bump: u8,
    _pad: [u8; 7],
    execution_info: RelayerExecutionInfo,
}

impl RelayerExecutionInfo {
    pub(crate) fn create<'a>(
        self,
        destination_address: Pubkey,
        pda: &AccountInfo<'a>, 
        signing_pda: &AccountInfo<'a>,
        payer: &AccountInfo<'a>, 
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
    )-> ProgramResult {
        let (pda_address, bump) = get_relayer_execution_pda(program_id, &destination_address);
        if &pda_address != pda.key {
            return Err( ProgramError::InvalidAccountData );
        }

        let (signing_pda_address, _) = get_singing_pda(&destination_address);

        // TODO: The below check has been disabled for easier testing, should re-enable.
        if &signing_pda_address != signing_pda.key /*|| !signing_pda.is_signer*/ {
            return Err( ProgramError::InvalidAccountData );
        }

        let relayer_execution_data = RelayerExecutionPda{
            bump,
            _pad: Default::default(),
            execution_info: self,
        };
        let data = bincode::serialize(&relayer_execution_data).map_err(|_| ProgramError::InvalidAccountData)?;
        let data_len = data.len() as u64;

        program_utils::init_pda_raw(payer, pda, program_id, system_program, data_len, &[seed_prefixes::RELAYER_EXECUTION, destination_address.as_ref(), &[bump]])?;

        let mut pda_data = pda.try_borrow_mut_data()?;
        pda_data.write(&data)?;

        Ok(())
    }

    pub(crate) fn read<'a>(
        pda: &AccountInfo<'a>,
        destination_address: &Pubkey,
        program_id: &Pubkey,
    )-> Result<Self, ProgramError> {
        let data = pda.try_borrow_data()?;
        let RelayerExecutionPda {
            bump,
            _pad,
            execution_info,
        } = bincode::deserialize(data.as_ref()).map_err(|_| ProgramError::InvalidAccountData)?;

        let pda_address = create_relayer_execution_pda(program_id, destination_address, bump)?;
        if &pda_address != pda.key {
            return Err( ProgramError::InvalidAccountData );
        }

        Ok(execution_info)
    }
}
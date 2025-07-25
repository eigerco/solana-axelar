//! Program state processor
use axelar_executable::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use interchain_token_transfer_gmp::{GMPPayload, SendToHub};
use itertools::{self, Itertools};
use program_utils::{pda::BorshPda, validate_system_account_key};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::sysvar::Sysvar;

use crate::processor::interchain_token::{self, DeployInterchainTokenAccounts};
use crate::processor::interchain_transfer::process_inbound_transfer;
use crate::processor::link_token;
use crate::state::token_manager::TokenManager;
use crate::state::InterchainTokenService;
use crate::{assert_its_not_paused, assert_valid_its_root_pda, Validate, ITS_HUB_CHAIN_NAME};
use crate::{instruction, FromAccountInfoSlice};

pub(crate) fn process_inbound<'a>(
    accounts: &'a [AccountInfo<'a>],
    message: Message,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    validate_with_gmp_metadata(gateway_accounts, &message)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let payload_account = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let its_root_pda_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_program.key)?;

    let its_root_config = InterchainTokenService::load(its_root_pda_account)?;
    assert_valid_its_root_pda(its_root_pda_account, its_root_config.bump)?;
    assert_its_not_paused(&its_root_config)?;

    if message.source_address != its_root_config.its_hub_address {
        msg!("Untrusted source address: {}", message.source_address);
        return Err(ProgramError::InvalidInstructionData);
    }

    let payload_account_data = payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**payload_account_data).try_into()?;

    let GMPPayload::ReceiveFromHub(inner) = GMPPayload::decode(message_payload.raw_payload)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("Unsupported GMP payload");
        return Err(ProgramError::InvalidInstructionData);
    };

    if !its_root_config.is_trusted_chain(&inner.source_chain) {
        msg!("Untrusted source chain: {}", inner.source_chain);
        return Err(ProgramError::InvalidInstructionData);
    }

    let payload =
        GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    validate_its_accounts(instruction_accounts, &payload)?;

    match payload {
        GMPPayload::InterchainTransfer(transfer) => process_inbound_transfer(
            message,
            payer,
            payload_account,
            instruction_accounts,
            &transfer,
            inner.source_chain,
        ),
        GMPPayload::DeployInterchainToken(deploy) => {
            let parsed_accounts =
                DeployInterchainTokenAccounts::from_account_info_slice(instruction_accounts, &())?;
            interchain_token::process_inbound_deploy(
                payer,
                parsed_accounts,
                deploy.token_id.0,
                deploy.name,
                deploy.symbol,
                deploy.decimals,
                0,
            )
        }
        GMPPayload::LinkToken(payload) => {
            link_token::process_inbound(payer, instruction_accounts, &payload)
        }
        GMPPayload::SendToHub(_)
        | GMPPayload::ReceiveFromHub(_)
        | GMPPayload::RegisterTokenMetadata(_) => Err(ProgramError::InvalidInstructionData),
    }
}

#[derive(Debug)]
pub(crate) struct GmpAccounts<'a> {
    pub(crate) gateway_root_account: &'a AccountInfo<'a>,
    pub(crate) _gateway_program_id: &'a AccountInfo<'a>,
    pub(crate) gas_service_config_account: &'a AccountInfo<'a>,
    pub(crate) _gas_service: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root_account: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing_account: &'a AccountInfo<'a>,
    pub(crate) program_account: &'a AccountInfo<'a>,
}

impl Validate for GmpAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for GmpAccounts<'a> {
    type Context = ();

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut accounts.iter();

        Ok(Self {
            gateway_root_account: next_account_info(accounts_iter)?,
            _gateway_program_id: next_account_info(accounts_iter)?,
            gas_service_config_account: next_account_info(accounts_iter)?,
            _gas_service: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root_account: next_account_info(accounts_iter)?,
            call_contract_signing_account: next_account_info(accounts_iter)?,
            program_account: next_account_info(accounts_iter)?,
        })
    }
}

pub(crate) fn process_outbound<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &GmpAccounts<'a>,
    payload: &GMPPayload,
    destination_chain: String,
    gas_value: u64,
    signing_pda_bump: u8,
    payload_hash: Option<[u8; 32]>,
    wrapped: bool,
) -> ProgramResult {
    let its_root_config = InterchainTokenService::load(accounts.its_root_account)?;
    assert_valid_its_root_pda(accounts.its_root_account, its_root_config.bump)?;
    assert_its_not_paused(&its_root_config)?;
    if !its_root_config.is_trusted_chain(&destination_chain)
        && destination_chain != ITS_HUB_CHAIN_NAME
    {
        msg!("Untrusted destination chain: {}", destination_chain);
        return Err(ProgramError::InvalidInstructionData);
    }

    let signing_pda =
        axelar_solana_gateway::create_call_contract_signing_pda(crate::ID, signing_pda_bump)?;

    if signing_pda.ne(accounts.call_contract_signing_account.key) {
        msg!("invalid call contract signing account / signing pda bump");
        return Err(ProgramError::InvalidAccountData);
    }

    let (payload_hash, call_contract_ix) = if let Some(payload_hash) = payload_hash {
        let ix = axelar_solana_gateway::instructions::call_contract_offchain_data(
            axelar_solana_gateway::id(),
            *accounts.gateway_root_account.key,
            crate::ID,
            Some((signing_pda, signing_pda_bump)),
            crate::ITS_HUB_CHAIN_NAME.to_owned(),
            its_root_config.its_hub_address.clone(),
            payload_hash,
        )?;

        (payload_hash, ix)
    } else {
        let payload = if wrapped {
            GMPPayload::SendToHub(SendToHub {
                selector: SendToHub::MESSAGE_TYPE_ID
                    .try_into()
                    .map_err(|_err| ProgramError::ArithmeticOverflow)?,
                destination_chain,
                payload: payload.encode().into(),
            })
            .encode()
        } else {
            payload.encode()
        };

        let payload_hash = solana_program::keccak::hashv(&[&payload]).to_bytes();
        let ix = axelar_solana_gateway::instructions::call_contract(
            axelar_solana_gateway::id(),
            *accounts.gateway_root_account.key,
            crate::ID,
            Some((signing_pda, signing_pda_bump)),
            crate::ITS_HUB_CHAIN_NAME.to_owned(),
            its_root_config.its_hub_address.clone(),
            payload,
        )?;

        (payload_hash, ix)
    };

    if gas_value > 0 {
        pay_gas(
            payer,
            accounts.gas_service_config_account,
            accounts.system_program,
            payload_hash,
            its_root_config.its_hub_address,
            gas_value,
        )?;
    }

    invoke_signed(
        &call_contract_ix,
        &[
            accounts.program_account.clone(),
            accounts.call_contract_signing_account.clone(),
            accounts.gateway_root_account.clone(),
        ],
        &[&[
            axelar_solana_gateway::seed_prefixes::CALL_CONTRACT_SIGNING_SEED,
            &[signing_pda_bump],
        ]],
    )?;

    Ok(())
}

fn pay_gas<'a>(
    payer: &'a AccountInfo<'a>,
    gas_service_config: &'a AccountInfo<'a>,
    system_program: &'a AccountInfo<'a>,
    payload_hash: [u8; 32],
    its_hub_address: String,
    gas_value: u64,
) -> ProgramResult {
    let gas_payment_ix =
        axelar_solana_gas_service::instructions::pay_native_for_contract_call_instruction(
            payer.key,
            crate::ITS_HUB_CHAIN_NAME.to_owned(),
            its_hub_address,
            payload_hash,
            *payer.key,
            vec![],
            gas_value,
        )?;

    invoke(
        &gas_payment_ix,
        &[
            payer.clone(),
            gas_service_config.clone(),
            system_program.clone(),
        ],
    )
}

fn validate_its_accounts(accounts: &[AccountInfo<'_>], payload: &GMPPayload) -> ProgramResult {
    const TOKEN_MANAGER_PDA_INDEX: usize = 2;
    const TOKEN_MINT_INDEX: usize = 3;
    const TOKEN_PROGRAM_INDEX: usize = 5;

    // In this case we cannot derive the mint account, so we just use what we got
    // and check later against the mint within the `TokenManager` PDA.
    let maybe_mint = if let GMPPayload::InterchainTransfer(_) = payload {
        accounts.get(TOKEN_MINT_INDEX).map(|account| *account.key)
    } else {
        None
    };

    let token_program = accounts
        .get(TOKEN_PROGRAM_INDEX)
        .map(|account| *account.key)
        .ok_or(ProgramError::InvalidAccountData)?;

    let derived_its_accounts = instruction::derive_its_accounts(
        payload,
        token_program,
        maybe_mint,
        Some(Clock::get()?.unix_timestamp),
    )?;

    for element in accounts.iter().zip_longest(derived_its_accounts.iter()) {
        match element {
            itertools::EitherOrBoth::Both(provided, derived) => {
                if provided.key != &derived.pubkey {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            itertools::EitherOrBoth::Left(_) | itertools::EitherOrBoth::Right(_) => {
                return Err(ProgramError::InvalidAccountData);
            }
        }
    }

    // Now we validate the mint account passed for `InterchainTransfer`
    if let Some(mint) = maybe_mint {
        let token_manager_pda = accounts
            .get(TOKEN_MANAGER_PDA_INDEX)
            .ok_or(ProgramError::InvalidAccountData)?;

        let token_manager = TokenManager::load(token_manager_pda)?;

        if token_manager.token_address.as_ref() != mint.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    Ok(())
}

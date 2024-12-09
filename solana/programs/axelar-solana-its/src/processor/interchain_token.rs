//! Module that handles the processing of the `InterchainToken` deployment.

use alloy_primitives::hex;
use axelar_solana_encoding::types::messages::Message;
use interchain_token_transfer_gmp::DeployInterchainToken;
use program_utils::BorshPda;
use role_management::processor::{ensure_signer_roles, RoleManagementAccounts};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};
use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_2022::extension::metadata_pointer::instruction::initialize as initialize_metadata_pointer;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensionsOwned};
use spl_token_2022::instruction::initialize_mint;
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::instruction::{initialize as initialize_metadata, update_field};
use spl_token_metadata_interface::state::{Field, TokenMetadata};

use super::token_manager::{DeployTokenManagerAccounts, DeployTokenManagerInternal};
use super::LocalAction;
use crate::instructions::{self, OptionalAccountsFlags};
use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{
    assert_valid_its_root_pda, assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice,
    Roles,
};

const TOKEN_ID_KEY: &str = "token_id";

impl LocalAction for DeployInterchainToken {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        _optional_accounts_flags: OptionalAccountsFlags,
        _message: Option<Message>,
    ) -> ProgramResult {
        process_deploy(payer, accounts, self)
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn process_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::interchain_token::Instruction,
) -> ProgramResult {
    match instruction {
        instructions::interchain_token::Instruction::Mint { amount } => {
            process_mint(accounts, amount)
        }
        instructions::interchain_token::Instruction::MinterInstruction(minter_instruction) => {
            process_minter_instruction(accounts, minter_instruction)
        }
    }
}

pub(crate) struct DeployInterchainTokenAccounts<'a> {
    pub(crate) gateway_root_pda: &'a AccountInfo<'a>,
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) its_roles_pda: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) minter: Option<&'a AccountInfo<'a>>,
    pub(crate) minter_roles_pda: Option<&'a AccountInfo<'a>>,
}

impl<'a> FromAccountInfoSlice<'a> for DeployInterchainTokenAccounts<'a> {
    type Context = ();
    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized,
    {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            gateway_root_pda: next_account_info(accounts_iter)?,
            system_account: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            its_roles_pda: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            minter: next_account_info(accounts_iter).ok(),
            minter_roles_pda: next_account_info(accounts_iter).ok(),
        })
    }
}

impl<'a> From<DeployInterchainTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    fn from(value: DeployInterchainTokenAccounts<'a>) -> Self {
        Self {
            gateway_root_pda: value.gateway_root_pda,
            system_account: value.system_account,
            its_root_pda: value.its_root_pda,
            token_manager_pda: value.token_manager_pda,
            token_mint: value.token_mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            its_roles_pda: value.its_roles_pda,
            _rent_sysvar: value.rent_sysvar,
            minter: value.minter,
            minter_roles_pda: value.minter_roles_pda,
            operator: value.minter,
            operator_roles_pda: value.minter_roles_pda,
        }
    }
}

/// Processes a [`DeployInterchainToken`] GMP message.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_deploy<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: DeployInterchainToken,
) -> ProgramResult {
    let parsed_accounts = DeployInterchainTokenAccounts::from_account_info_slice(accounts, &())?;
    let its_root_pda_bump = InterchainTokenService::load(parsed_accounts.its_root_pda)?.bump;
    assert_valid_its_root_pda(
        parsed_accounts.its_root_pda,
        parsed_accounts.gateway_root_pda.key,
        its_root_pda_bump,
    )?;

    let (interchain_token_pda, interchain_token_pda_bump) = crate::find_interchain_token_pda(
        parsed_accounts.its_root_pda.key,
        payload.token_id.as_ref(),
    );
    if interchain_token_pda.ne(parsed_accounts.token_mint.key) {
        msg!("Invalid mint account provided");
        return Err(ProgramError::InvalidArgument);
    }

    let (token_manager_pda, token_manager_pda_bump) =
        crate::find_token_manager_pda(parsed_accounts.its_root_pda.key, &payload.token_id);
    if token_manager_pda.ne(parsed_accounts.token_manager_pda.key) {
        msg!("Invalid TokenManager account provided");
        return Err(ProgramError::InvalidArgument);
    }

    setup_mint(
        payer,
        &parsed_accounts,
        payload.decimals,
        &payload.token_id.0,
        interchain_token_pda_bump,
    )?;
    setup_metadata(
        payer,
        &parsed_accounts,
        &payload.token_id.0,
        payload.name,
        payload.symbol,
        String::new(),
        token_manager_pda_bump,
    )?;

    // The minter passed in the DeployInterchainToken call is used as the
    // `TokenManager` operator as well, see:
    // https://github.com/axelarnetwork/interchain-token-service/blob/v2.0.1/contracts/InterchainTokenService.sol#L758
    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager::Type::NativeInterchainToken,
        payload.token_id.0,
        *parsed_accounts.token_mint.key,
        parsed_accounts.minter.map(|account| *account.key),
        parsed_accounts.minter.map(|account| *account.key),
    );

    let deploy_token_manager_accounts = DeployTokenManagerAccounts::from(parsed_accounts);
    super::token_manager::deploy(
        payer,
        &deploy_token_manager_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    Ok(())
}

fn process_mint<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let mint = next_account_info(accounts_iter)?;
    let destination_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let minter = next_account_info(accounts_iter)?;
    let minter_roles_pda = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    let token_manager = TokenManager::load(token_manager_pda)?;
    assert_valid_token_manager_pda(
        token_manager_pda,
        its_root_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    if token_manager.token_address.as_ref() != mint.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    if mint.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }

    ensure_signer_roles(
        &crate::id(),
        token_manager_pda,
        minter,
        minter_roles_pda,
        Roles::MINTER,
    )?;

    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program.key,
            mint.key,
            destination_account.key,
            token_manager_pda.key,
            &[],
            amount,
        )?,
        &[
            mint.clone(),
            destination_account.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.key.as_ref(),
            token_manager.token_id.as_ref(),
            &[token_manager.bump],
        ]],
    )?;
    Ok(())
}

fn setup_mint<'a>(
    payer: &AccountInfo<'a>,
    accounts: &DeployInterchainTokenAccounts<'a>,
    decimals: u8,
    token_id: &[u8],
    interchain_token_pda_bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let account_size =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])?;

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            accounts.token_mint.key,
            rent.minimum_balance(account_size).max(1),
            account_size
                .try_into()
                .map_err(|_err| ProgramError::InvalidAccountData)?,
            accounts.token_program.key,
        ),
        &[
            payer.clone(),
            accounts.token_mint.clone(),
            accounts.system_account.clone(),
            accounts.token_program.clone(),
            accounts.token_manager_pda.clone(),
        ],
        &[&[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[interchain_token_pda_bump],
        ]],
    )?;

    invoke(
        &initialize_metadata_pointer(
            &spl_token_2022::id(),
            accounts.token_mint.key,
            Some(*accounts.token_manager_pda.key),
            Some(*accounts.token_mint.key),
        )?,
        &[
            payer.clone(),
            accounts.token_mint.clone(),
            accounts.token_manager_pda.clone(),
        ],
    )?;

    invoke(
        &initialize_mint(
            &spl_token_2022::id(),
            accounts.token_mint.key,
            accounts.token_manager_pda.key,
            Some(accounts.token_manager_pda.key),
            decimals,
        )?,
        &[
            accounts.token_mint.clone(),
            accounts.rent_sysvar.clone(),
            accounts.token_manager_pda.clone(),
            accounts.token_program.clone(),
        ],
    )?;

    Ok(())
}

fn setup_metadata<'a>(
    payer: &AccountInfo<'a>,
    accounts: &DeployInterchainTokenAccounts<'a>,
    token_id: &[u8],
    name: String,
    symbol: String,
    uri: String,
    token_manager_pda_bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let token_metadata = TokenMetadata {
        update_authority: OptionalNonZeroPubkey(*accounts.token_manager_pda.key),
        name,
        symbol,
        uri,
        mint: *accounts.token_mint.key,
        additional_metadata: vec![(TOKEN_ID_KEY.to_owned(), hex::encode(token_id))],
    };

    let mint_state =
        StateWithExtensionsOwned::<Mint>::unpack(accounts.token_mint.try_borrow_data()?.to_vec())?;
    let account_lamports = accounts.token_mint.lamports();
    let new_account_len = mint_state
        .try_get_new_account_len_for_variable_len_extension::<TokenMetadata>(&token_metadata)?;
    let new_rent_exemption_minimum = rent.minimum_balance(new_account_len);
    let additional_lamports = new_rent_exemption_minimum.saturating_sub(account_lamports);

    invoke(
        &system_instruction::transfer(payer.key, accounts.token_mint.key, additional_lamports),
        &[
            payer.clone(),
            accounts.token_mint.clone(),
            accounts.system_account.clone(),
        ],
    )?;

    invoke_signed(
        &initialize_metadata(
            &spl_token_2022::id(),
            accounts.token_mint.key,
            accounts.token_manager_pda.key,
            accounts.token_mint.key,
            accounts.token_manager_pda.key,
            token_metadata.name,
            token_metadata.symbol,
            token_metadata.uri,
        ),
        &[
            accounts.token_mint.clone(),
            accounts.token_manager_pda.clone(),
            accounts.token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[token_manager_pda_bump],
        ]],
    )?;

    invoke_signed(
        &update_field(
            &spl_token_2022::id(),
            accounts.token_mint.key,
            accounts.token_manager_pda.key,
            Field::Key(TOKEN_ID_KEY.to_owned()),
            hex::encode(token_id),
        ),
        &[
            accounts.token_mint.clone(),
            accounts.token_manager_pda.clone(),
            accounts.token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[token_manager_pda_bump],
        ]],
    )?;

    Ok(())
}

fn process_minter_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::minter::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let its_root_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;
    let token_manager = TokenManager::load(role_management_accounts.resource)?;
    assert_valid_token_manager_pda(
        role_management_accounts.resource,
        its_root_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    match instruction {
        instructions::minter::Instruction::TransferMintership(inputs) => {
            role_management::processor::transfer(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::MINTER,
            )?;
        }
        instructions::minter::Instruction::ProposeMintership(inputs) => {
            role_management::processor::propose(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::MINTER,
            )?;
        }
        instructions::minter::Instruction::AcceptMintership(inputs) => {
            role_management::processor::accept(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::empty(),
            )?;
        }
    }

    Ok(())
}

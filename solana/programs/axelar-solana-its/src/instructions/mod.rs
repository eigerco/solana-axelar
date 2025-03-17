#![allow(clippy::too_many_arguments)]
//! Instructions supported by the ITS program.

use std::borrow::Cow;

use axelar_message_primitives::{DataPayload, DestinationProgramId};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::incoming_message::command_id;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::{GMPPayload, InterchainTransfer, SendToHub};
use role_management::instructions::RoleManagementInstruction;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use typed_builder::TypedBuilder;

use crate::state::{self, flow_limit};
use crate::Roles;

pub mod interchain_token;
pub mod minter;
pub mod operator;
pub mod token_manager;

/// Instructions supported by the ITS program.
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] Program data account
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root account
    /// 4. [] System program account
    /// 5. [] The account that will become the operator of the ITS
    /// 6. [writable] The address of the account that will store the roles of the operator account.
    Initialize {
        /// The name of the chain the ITS is running on.
        chain_name: String,

        /// The address of the ITS Hub
        its_hub_address: String,
    },

    /// Pauses or unpauses the interchain token service.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    SetPauseStatus {
        /// The new pause status.
        paused: bool,
    },
    /// Sets a chain as trusted, allowing communication between this ITS and the ITS of that chain.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    /// 4. [] The system program account.
    SetTrustedChain {
        /// The name of the chain to be trusted.
        chain_name: String,
    },

    /// Unsets a chain as trusted, disallowing communication between this ITS and the ITS of that chain.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    /// 4. [] The system program account.
    RemoveTrustedChain {
        /// The name of the chain from which trust is removed.
        chain_name: String,
    },

    /// Approves the deployment of remote token with a destination minter
    ///
    /// 0. [writable,signer] The address of the payer, needs to have minter role on the token
    ///    manager.
    /// 1. [] The token manager account associated with the token
    /// 2. [] The account that holds the payer roles on the token manager
    /// 3. [writable] The account that will hold the approval of the deployment
    /// 4. [] The system program account
    ApproveDeployRemoteInterchainToken {
        /// The address of the account that deployed the `InterchainToken`
        deployer: Pubkey,
        /// The salt used to deploy the `InterchainToken`
        salt: [u8; 32],
        /// The remote chain where the `InterchainToken` will be deployed.
        destination_chain: String,
        /// The approved address of the minter on the destination chain
        destination_minter: Vec<u8>,
    },

    /// Revokes an approval of a deployment of remote token with a destination minter
    ///
    /// 0. [writable,signer] The address of the payer, needs to have minter role on the token
    ///    manager.
    /// 1. [] The token manager account associated with the token
    /// 2. [] The account that holds the payer roles on the token manager
    /// 3. [writable] The account holding the approval of the deployment that should be revoked
    /// 4. [] The system program account
    RevokeDeployRemoteInterchainToken {
        /// The address of the account that deployed the `InterchainToken`
        deployer: Pubkey,
        /// The salt used to deploy the `InterchainToken`
        salt: [u8; 32],
        /// The remote chain where the `InterchainToken` would be deployed.
        destination_chain: String,
    },

    /// Registers a canonical token as an interchain token and deploys its token manager.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account derived from the `token_id` that will be initialized
    /// 6. [] The mint account (token address) of the original token
    /// 7. [] The token manager Associated Token Account
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [] The rent sysvar account
    /// 11. [] The Metaplex metadata program account (`mpl_token_metadata`)
    RegisterCanonicalInterchainToken,

    /// Deploys a canonical interchain token on a remote chain.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account associated with the interchain token
    /// 6. [writable] The mint account (token address) to deploy
    /// 7. [writable] The token manager Associated Token Account associated with the mint
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [writable] The account holding the roles of the deployer on the ITS root account
    /// 11. [] The rent sysvar account
    /// 12. [] Optional account to set as operator on the `TokenManager`.
    /// 13. [writable] In case an operator is being set, this should be the account holding the roles of
    ///     the operator on the `TokenManager`
    DeployRemoteCanonicalInterchainToken {
        /// The remote chain where the `InterchainToken` should be deployed.
        destination_chain: String,
        /// The gas amount to be sent for deployment.
        gas_value: u64,
        /// The bump from the call contract signing account PDA derivation
        signing_pda_bump: u8,
    },

    /// Transfers interchain tokens.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    InterchainTransfer {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// The bump from the call contract signing account PDA derivation
        signing_pda_bump: u8,
    },

    /// Deploys an interchain token.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The GMP gateway root account
    /// 2. [] The system program account
    /// 3. [] The ITS root account
    /// 4. [writable] The token manager account associated with the interchain token
    /// 5. [writable] The mint account (token address) to deploy
    /// 6. [writable] The token manager Associated Token Account associated with the mint
    /// 7. [] The token program account (`spl_token_2022`)
    /// 8. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 9. [writable] The account holding the roles of the deployer on the ITS root account
    /// 10. [] The rent sysvar account
    /// 11. [] The instructions sysvar account
    /// 12. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 13. [writable] The Metaplex metadata account associated with the mint
    /// 14. [] The account to set as minter of the token
    /// 15. [writable] The account holding the roles of the minter account on the `TokenManager`
    DeployInterchainToken {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// Token name
        name: String,

        /// Token symbol
        symbol: String,

        /// Token decimals
        decimals: u8,
    },

    /// Deploys a remote interchain token
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The Metaplex metadata account associated with the mint
    /// 3. [] The instructions sysvar account
    /// 4. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 5. [] The GMP gateway root account
    /// 6. [] The GMP gateway program account
    /// 7. [writable] The GMP gas configuration account
    /// 8. [] The GMP gas service program account
    /// 9. [] The system program account
    /// 10. [] The ITS root account
    /// 11. [] The GMP call contract signing account
    /// 12. [] The ITS program account
    DeployRemoteInterchainToken {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// The chain where the `InterchainToken` should be deployed.
        destination_chain: String,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Deploys a remote interchain token with associated minter
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The Metaplex metadata account associated with the mint
    /// 3. [] The account of the minter that approved the deployment
    /// 4. [writable] The account holding the approval for the deployment
    /// 5. [] The account holding the roles of the minter on the token manager associated with the
    ///    interchain token
    /// 6. [] The token manager account associated with the interchain token
    /// 7. [] The instructions sysvar account
    /// 8. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 9. [] The GMP gateway root account
    /// 10. [] The GMP gateway program account
    /// 11. [writable] The GMP gas configuration account
    /// 12. [] The GMP gas service program account
    /// 13. [] The system program account
    /// 14. [] The ITS root account
    /// 15. [] The GMP call contract signing account
    /// 16. [] The ITS program account
    DeployRemoteInterchainTokenWithMinter {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// The chain where the `InterchainToken` should be deployed.
        destination_chain: String,

        /// The minter on the destination chain
        destination_minter: Vec<u8>,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Registers token metadata.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 3. [] The GMP gateway root account
    /// 4. [] The GMP gateway program account
    /// 5. [writable] The GMP gas configuration account
    /// 6. [] The GMP gas service program account
    /// 7. [] The system program account
    /// 8. [] The ITS root account
    /// 9. [] The GMP call contract signing account
    /// 10. [] The ITS program account
    RegisterTokenMetadata {
        /// The gas value to be paid for the GMP transaction
        gas_value: u64,
        /// The signing PDA bump
        signing_pda_bump: u8,
    },

    /// Registers a custom token with ITS, deploying a new [`TokenManager`] to manage it.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account associated with the interchain token
    /// 6. [writable] The mint account (token address) to deploy
    /// 7. [writable] The token manager Associated Token Account associated with the mint
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [writable] The account holding the roles of the deployer on the ITS root account
    /// 11. [] The rent sysvar account
    /// 12. [] Optional account to set as operator on the `TokenManager`.
    /// 13. [writable] In case an operator is being set, this should be the account holding the roles of
    ///     the operator on the `TokenManager`
    RegisterCustomToken {
        /// Salt used to derive the `token_id` associated with the token.
        salt: [u8; 32],
        /// The token manager type.
        token_manager_type: state::token_manager::Type,
        /// The operator account
        operator: Option<Pubkey>,
    },

    /// Link a local token derived from salt and payer to another token on the `destination_chain`,
    /// at the `destination_token_address`.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The `TokenManager` account associated with the token being linked
    /// 2. [] The GMP gateway root account
    /// 3. [] The GMP gateway program account
    /// 4. [writable] The GMP gas configuration account
    /// 5. [] The GMP gas service program account
    /// 6. [] The system program account
    /// 7. [] The ITS root account
    /// 8. [] The GMP call contract signing account
    /// 9. [] The ITS program account
    LinkToken {
        /// Salt used to derive the `token_id` associated with the token.
        salt: [u8; 32],
        /// The chain where the token is being linked to.
        destination_chain: String,
        /// The address of the token on the destination chain.
        destination_token_address: Vec<u8>,
        /// The type of token manager used on the destination chain.
        token_manager_type: state::token_manager::Type,
        /// The params required on the destination chain.
        link_params: Vec<u8>,
        /// The gas value to be paid for the GMP transaction
        gas_value: u64,
        /// The signing PDA bump
        signing_pda_bump: u8,
    },

    /// Transfers tokens to a contract on the destination chain and call the give instruction on
    /// it. This instruction is is the same as [`InterchainTransfer`], but will fail if call data
    /// is empty.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    CallContractWithInterchainToken {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// Call data
        data: Vec<u8>,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Transfers tokens to a contract on the destination chain and call the give instruction on
    /// it. This instruction is is the same as [`InterchainTransfer`], but will fail if call data
    /// is empty.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    CallContractWithInterchainTokenOffchainData {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// Hash of the entire payload
        payload_hash: [u8; 32],

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    ///
    /// 3..N Accounts depend on the inner ITS instruction.
    ItsGmpPayload {
        /// The GMP metadata
        message: Message,
    },

    /// Sets the flow limit for an interchain token.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The ITS root account
    /// 2. [writable] The token manager account associated with the interchain token
    /// 3. [writable] The account holding the roles of the payer on the ITS root account
    /// 4. [writable] The account holding the roles of the payer on the `TokenManager`
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: u64,
    },

    /// ITS operator role management instructions.
    ///
    /// 0. [] Gateway root pda
    /// 1..N [`operator::OperatorInstruction`] accounts, where the resource is
    /// the ITS root PDA.
    OperatorInstruction(operator::Instruction),

    /// Instructions operating on deployed [`TokenManager`] instances.
    TokenManagerInstruction(token_manager::Instruction),

    /// Instructions operating in Interchain Tokens.
    InterchainTokenInstruction(interchain_token::Instruction),
}

/// Inputs for the [`its_gmp_payload`] function.
///
/// To construct this type, use its builder API.
///
/// # Example
///
/// ```ignore
/// use axelar_solana_its::instructions::ItsGmpInstructionInputs;
///
/// let inputs = ItsGmpInstructionInputs::builder()
///   .payer(payer_pubkey)
///   .incoming_message_pda(gateway_approved_message_pda)
///   .message(message)
///   .payload(payload)
///   .token_program(spl_token_2022::ID)
///   .mint(mint_pubkey)
///   .bumps(bumps)
///   .build();
/// ```
#[derive(Debug, Clone, TypedBuilder)]
pub struct ItsGmpInstructionInputs {
    /// The payer account.
    pub(crate) payer: Pubkey,

    /// The PDA used to track the message status by the gateway program.
    pub(crate) incoming_message_pda: Pubkey,

    /// The PDA used to to store the message payload.
    pub(crate) message_payload_pda: Pubkey,

    /// The Axelar GMP metadata.
    pub(crate) message: Message,

    /// The ITS GMP payload.
    pub(crate) payload: GMPPayload,

    /// The token program required by the instruction (spl-token or
    /// spl-token-2022).
    pub(crate) token_program: Pubkey,

    /// The mint account required by the instruction. Hard requirement for
    /// `InterchainTransfer` instruction. Optional for `DeployTokenManager` and
    /// ignored by `DeployInterchainToken`.
    #[builder(default, setter(strip_option(fallback = mint_opt)))]
    pub(crate) mint: Option<Pubkey>,

    /// The current approximate timestamp. Required for `InterchainTransfer`s.
    #[builder(default, setter(strip_option(fallback = timestamp_opt)))]
    pub(crate) timestamp: Option<i64>,
}

/// Creates an [`InterchainTokenServiceInstruction::Initialize`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn initialize(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    operator: Pubkey,
    chain_name: String,
    its_hub_address: String,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &operator);

    let data = to_vec(&InterchainTokenServiceInstruction::Initialize {
        chain_name,
        its_hub_address,
    })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(operator, false),
        AccountMeta::new(user_roles_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::SetPauseStatus`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_pause_status(payer: Pubkey, paused: bool) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetPauseStatus { paused })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::SetTrustedChain`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_trusted_chain(payer: Pubkey, chain_name: String) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetTrustedChain { chain_name })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RemoveTrustedChain`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn remove_trusted_chain(
    payer: Pubkey,
    chain_name: String,
) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::RemoveTrustedChain { chain_name })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken`] instruction.
///
/// Allow the minter to approve the deployer for a remote interchain token deployment that uses a
/// custom `destination_minter` address. This ensures that a token deployer can't choose the
/// `destination_minter` itself, and requires the approval of the minter to reduce trust assumptions
/// on the deployer.
///
/// # Parameters
///
/// `payer`: The account paying for the transaction, also with minter role on the `TokenManager`.
/// `deployer`: The address of the account that deployed the `InterchainToken`.
/// `salt`: The unique salt for deploying the token.
/// `destination_chain`: The name of the destination chain.
/// `destination_minter`: The minter address to set on the deployed token on the destination chain. This can be arbitrary bytes since the encoding of the account is dependent on the destination chain.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn approve_deploy_remote_interchain_token(
    payer: Pubkey,
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    destination_minter: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &payer);
    let (deploy_approval_pda, _) =
        crate::find_deployment_approval_pda(&payer, &token_id, &destination_chain);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(roles_pda, false),
        AccountMeta::new(deploy_approval_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
            destination_minter,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken`] instruction.
///
/// Allows the minter to revoke a deployer's approval for a remote interchain token deployment that
/// uses a custom `destination_minter` address.
///
/// # Parameters
///
/// `payer`: The account paying for the transaction, also with minter role on the `TokenManager`.
/// `deployer`: The address of the account that deployed the `InterchainToken`.
/// `salt`: The unique salt for deploying the token.
/// `destination_chain`: The name of the destination chain.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn revoke_deploy_remote_interchain_token(
    payer: Pubkey,
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &payer);
    let (deploy_approval_pda, _) =
        crate::find_deployment_approval_pda(&payer, &token_id, &destination_chain);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(roles_pda, false),
        AccountMeta::new(deploy_approval_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_canonical_interchain_token(
    payer: Pubkey,
    mint: Pubkey,
    token_program: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::canonical_interchain_token_id(&mint);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (token_metadata_account, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_metadata_account, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_canonical_interchain_token(
    payer: Pubkey,
    mint: Pubkey,
    destination_chain: String,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(gas_service, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteCanonicalInterchainToken {
            destination_chain,
            gas_value,
            signing_pda_bump,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_interchain_token(
    payer: Pubkey,
    salt: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    minter: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint,
        &spl_token_2022::ID,
    );
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &minter);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(spl_token_2022::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new(metadata_account_key, false),
        AccountMeta::new_readonly(minter, false),
        AccountMeta::new(minter_roles_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::DeployInterchainToken {
        salt,
        name,
        symbol,
        decimals,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_interchain_token(
    payer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(gas_service, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_interchain_token_with_minter(
    payer: Pubkey,
    salt: [u8; 32],
    minter: Pubkey,
    destination_chain: String,
    destination_minter: Vec<u8>,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (deploy_approval, _) =
        crate::find_deployment_approval_pda(&minter, &token_id, &destination_chain);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &minter);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(minter, false),
        AccountMeta::new(deploy_approval, false),
        AccountMeta::new_readonly(minter_roles_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(gas_service, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
            destination_minter,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates [`InterchainTokenServiceInstruction::RegisterTokenMetadata`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_token_metadata(
    payer: Pubkey,
    mint: Pubkey,
    token_program: Pubkey,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(gas_service, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterTokenMetadata {
        gas_value,
        signing_pda_bump,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RegisterCustomToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_custom_token(
    payer: Pubkey,
    salt: [u8; 32],
    mint: Pubkey,
    token_manager_type: state::token_manager::Type,
    token_program: Pubkey,
    operator: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let token_id = crate::linked_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (token_metadata_account, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_metadata_account, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    if let Some(operator) = operator {
        let (operator_roles_pda, _) =
            role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &operator);
        accounts.push(AccountMeta::new(operator, false));
        accounts.push(AccountMeta::new(operator_roles_pda, false));
    }

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterCustomToken {
        salt,
        token_manager_type,
        operator,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::LinkToken`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn link_token(
    payer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    destination_token_address: Vec<u8>,
    token_manager_type: state::token_manager::Type,
    link_params: Vec<u8>,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let token_id = crate::linked_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(gas_service, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::LinkToken {
        salt,
        destination_chain,
        destination_token_address,
        token_manager_type,
        link_params,
        gas_value,
        signing_pda_bump,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::InterchainTransfer`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn interchain_transfer(
    payer: Pubkey,
    source_account: Pubkey,
    authority: Option<Pubkey>,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    token_program: Pubkey,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
    timestamp: i64,
) -> Result<Instruction, ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(
        payer,
        source_account,
        authority,
        token_id,
        mint,
        token_program,
        gas_service,
        gas_config_pda,
        timestamp,
    )?;

    let data = to_vec(&InterchainTokenServiceInstruction::InterchainTransfer {
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
        signing_pda_bump,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CallContractWithInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn call_contract_with_interchain_token(
    payer: Pubkey,
    source_account: Pubkey,
    authority: Option<Pubkey>,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    data: Vec<u8>,
    token_program: Pubkey,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
    timestamp: i64,
) -> Result<Instruction, ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(
        payer,
        source_account,
        authority,
        token_id,
        mint,
        token_program,
        gas_service,
        gas_config_pda,
        timestamp,
    )?;

    let data = to_vec(
        &InterchainTokenServiceInstruction::CallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            data,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn call_contract_with_interchain_token_offchain_data(
    payer: Pubkey,
    source_account: Pubkey,
    authority: Option<Pubkey>,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    data: Vec<u8>,
    token_program: Pubkey,
    gas_value: u64,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
    timestamp: i64,
) -> Result<(Instruction, Vec<u8>), ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(
        payer,
        source_account,
        authority,
        token_id,
        mint,
        token_program,
        gas_service,
        gas_config_pda,
        timestamp,
    )?;

    let payload = GMPPayload::SendToHub(SendToHub {
        payload: GMPPayload::InterchainTransfer(InterchainTransfer {
            selector: InterchainTransfer::MESSAGE_TYPE_ID
                .try_into()
                .map_err(|_err| ProgramError::ArithmeticOverflow)?,
            token_id: token_id.into(),
            source_address: source_account.to_bytes().into(),
            destination_address: destination_address.clone().into(),
            amount: alloy_primitives::U256::from(amount),
            data: data.into(),
        })
        .encode()
        .into(),
        selector: SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        destination_chain: destination_chain.clone(),
    })
    .encode();

    let data = to_vec(
        &InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            payload_hash: solana_program::keccak::hashv(&[&payload]).0,
        },
    )?;

    Ok((
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        },
        payload,
    ))
}

fn interchain_transfer_accounts(
    payer: Pubkey,
    source_account: Pubkey,
    authority: Option<Pubkey>,
    token_id: [u8; 32],
    mint: Pubkey,
    token_program: Pubkey,
    gas_service: Pubkey,
    gas_config_pda: Pubkey,
    timestamp: i64,
) -> Result<(Vec<AccountMeta>, u8), ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let flow_epoch = flow_limit::flow_epoch_with_timestamp(timestamp)?;
    let (flow_slot_pda, _) = crate::find_flow_slot_pda(&token_manager_pda, flow_epoch);
    let (authority, signer) = authority.map_or((token_manager_pda, false), |key| (key, true));
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);

    Ok((
        vec![
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(authority, signer),
            AccountMeta::new(source_account, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(token_manager_pda, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new(flow_slot_pda, false),
            AccountMeta::new_readonly(gateway_root_pda, false),
            AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
            AccountMeta::new(gas_config_pda, false),
            AccountMeta::new_readonly(gas_service, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new_readonly(call_contract_signing_pda, false),
            AccountMeta::new_readonly(crate::ID, false),
        ],
        signing_pda_bump,
    ))
}

/// Creates an [`InterchainTokenServiceInstruction::SetFlowLimit`].
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_flow_limit(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limit: u64,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) =
        crate::find_its_root_pda(&axelar_solana_gateway::get_gateway_root_config_pda().0);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &payer);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetFlowLimit { flow_limit })?;
    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(its_user_roles_pda, false),
        AccountMeta::new_readonly(token_manager_user_roles_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ItsGmpPayload`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn its_gmp_payload(inputs: ItsGmpInstructionInputs) -> Result<Instruction, ProgramError> {
    let mut accounts = prefix_accounts(
        &inputs.payer,
        &inputs.incoming_message_pda,
        &inputs.message_payload_pda,
        &inputs.message,
    );
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();

    let unwrapped_payload = match inputs.payload {
        GMPPayload::InterchainTransfer(_)
        | GMPPayload::DeployInterchainToken(_)
        | GMPPayload::LinkToken(_)
        | GMPPayload::RegisterTokenMetadata(_) => inputs.payload,
        GMPPayload::SendToHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
        GMPPayload::ReceiveFromHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    };

    let mut its_accounts = derive_its_accounts(
        &unwrapped_payload,
        gateway_root_pda,
        inputs.token_program,
        inputs.mint,
        inputs.timestamp,
    )?;

    accounts.append(&mut its_accounts);

    let data = to_vec(&InterchainTokenServiceInstruction::ItsGmpPayload {
        message: inputs.message,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::TransferOperatorship`]
/// variant.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn transfer_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::transfer_operatorship(payer, its_root_pda, to, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::ProposeOperatorship`] variant.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn propose_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::propose_operatorship(payer, its_root_pda, to, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::AcceptOperatorship`] variant.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn accept_operatorship(payer: Pubkey, from: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::accept_operatorship(payer, its_root_pda, from, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

fn prefix_accounts(
    payer: &Pubkey,
    gateway_incoming_message_pda: &Pubkey,
    gateway_message_payload_pda: &Pubkey,
    message: &Message,
) -> Vec<AccountMeta> {
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let destination_program = DestinationProgramId(crate::ID);
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*gateway_incoming_message_pda, false),
        AccountMeta::new_readonly(*gateway_message_payload_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
    ]
}

pub(crate) fn derive_its_accounts<'a, T>(
    payload: T,
    gateway_root_pda: Pubkey,
    token_program: Pubkey,
    maybe_mint: Option<Pubkey>,
    maybe_timestamp: Option<i64>,
) -> Result<Vec<AccountMeta>, ProgramError>
where
    T: TryInto<ItsMessageRef<'a>>,
{
    let message: ItsMessageRef<'_> = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;
    if let ItsMessageRef::DeployInterchainToken { .. } = message {
        if token_program != spl_token_2022::ID {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let (mut accounts, mint, token_manager_pda) =
        derive_common_its_accounts(gateway_root_pda, token_program, &message, maybe_mint)?;

    let mut message_specific_accounts = derive_specific_its_accounts(
        &message,
        mint,
        token_manager_pda,
        token_program,
        maybe_timestamp,
    )?;

    accounts.append(&mut message_specific_accounts);

    Ok(accounts)
}

fn derive_specific_its_accounts(
    message: &ItsMessageRef<'_>,
    mint_account: Pubkey,
    token_manager_pda: Pubkey,
    token_program: Pubkey,
    maybe_timestamp: Option<i64>,
) -> Result<Vec<AccountMeta>, ProgramError> {
    let mut specific_accounts = Vec::new();

    match message {
        ItsMessageRef::InterchainTransfer {
            destination_address,
            data,
            ..
        } => {
            let destination_account = Pubkey::new_from_array(
                (*destination_address)
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            );
            let Some(timestamp) = maybe_timestamp else {
                return Err(ProgramError::InvalidInstructionData);
            };
            let epoch = crate::state::flow_limit::flow_epoch_with_timestamp(timestamp)?;
            let (flow_slot_pda, _) = crate::find_flow_slot_pda(&token_manager_pda, epoch);

            specific_accounts.push(AccountMeta::new(destination_account, false));
            specific_accounts.push(AccountMeta::new(flow_slot_pda, false));

            if !data.is_empty() {
                let execute_data = DataPayload::decode(data)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;
                let (metadata_account_key, _) =
                    mpl_token_metadata::accounts::Metadata::find_pda(&mint_account);
                let program_ata = get_associated_token_address_with_program_id(
                    &destination_account,
                    &mint_account,
                    &token_program,
                );

                specific_accounts.push(AccountMeta::new(program_ata, false));
                specific_accounts.push(AccountMeta::new_readonly(mpl_token_metadata::ID, false));
                specific_accounts.push(AccountMeta::new(metadata_account_key, false));
                specific_accounts.extend(execute_data.account_meta().iter().cloned());
            }
        }
        ItsMessageRef::DeployInterchainToken { minter, .. } => {
            let (metadata_account_key, _) =
                mpl_token_metadata::accounts::Metadata::find_pda(&mint_account);

            specific_accounts.push(AccountMeta::new_readonly(sysvar::instructions::ID, false));
            specific_accounts.push(AccountMeta::new_readonly(mpl_token_metadata::ID, false));
            specific_accounts.push(AccountMeta::new(metadata_account_key, false));

            if minter.len() == axelar_solana_encoding::types::pubkey::ED25519_PUBKEY_LEN {
                let minter_key = Pubkey::new_from_array(
                    (*minter)
                        .try_into()
                        .map_err(|_err| ProgramError::InvalidInstructionData)?,
                );
                let (minter_roles_pda, _) = role_management::find_user_roles_pda(
                    &crate::ID,
                    &token_manager_pda,
                    &minter_key,
                );

                specific_accounts.push(AccountMeta::new_readonly(minter_key, false));
                specific_accounts.push(AccountMeta::new(minter_roles_pda, false));
            }
        }
        ItsMessageRef::LinkToken { link_params, .. } => {
            if let Ok(operator) = Pubkey::try_from(*link_params) {
                let (operator_roles_pda, _) =
                    role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &operator);

                specific_accounts.push(AccountMeta::new_readonly(operator, false));
                specific_accounts.push(AccountMeta::new(operator_roles_pda, false));
            }
        }
    };

    Ok(specific_accounts)
}

fn try_retrieve_mint(
    interchain_token_pda: &Pubkey,
    payload: &ItsMessageRef<'_>,
    maybe_mint: Option<Pubkey>,
) -> Result<Pubkey, ProgramError> {
    if let Some(mint) = maybe_mint {
        return Ok(mint);
    }

    match payload {
        ItsMessageRef::LinkToken {
            destination_token_address,
            ..
        } => Pubkey::try_from(*destination_token_address)
            .map_err(|_err| ProgramError::InvalidInstructionData),
        ItsMessageRef::InterchainTransfer { .. } => {
            maybe_mint.ok_or(ProgramError::InvalidInstructionData)
        }
        ItsMessageRef::DeployInterchainToken { .. } => Ok(*interchain_token_pda),
    }
}

fn derive_common_its_accounts(
    gateway_root_pda: Pubkey,
    token_program: Pubkey,
    message: &ItsMessageRef<'_>,
    maybe_mint: Option<Pubkey>,
) -> Result<(Vec<AccountMeta>, Pubkey, Pubkey), ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) =
        crate::find_interchain_token_pda(&its_root_pda, message.token_id());
    let token_mint = try_retrieve_mint(&interchain_token_pda, message, maybe_mint)?;
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, message.token_id());

    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &token_mint,
        &token_program,
    );

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    Ok((
        vec![
            AccountMeta::new_readonly(gateway_root_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new(token_manager_pda, false),
            AccountMeta::new(token_mint, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            AccountMeta::new(its_user_roles_pda, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        token_mint,
        token_manager_pda,
    ))
}

#[allow(dead_code)]
pub(crate) enum ItsMessageRef<'a> {
    InterchainTransfer {
        token_id: Cow<'a, [u8; 32]>,
        source_address: &'a [u8],
        destination_address: &'a [u8],
        amount: u64,
        data: &'a [u8],
    },
    DeployInterchainToken {
        token_id: Cow<'a, [u8; 32]>,
        name: &'a str,
        symbol: &'a str,
        decimals: u8,
        minter: &'a [u8],
    },
    LinkToken {
        token_id: Cow<'a, [u8; 32]>,
        source_token_address: &'a [u8],
        destination_token_address: &'a [u8],
        token_manager_type: state::token_manager::Type,
        link_params: &'a [u8],
    },
}

impl ItsMessageRef<'_> {
    /// Returns the token id for the message.
    pub(crate) fn token_id(&self) -> &[u8; 32] {
        match self {
            ItsMessageRef::InterchainTransfer { token_id, .. }
            | ItsMessageRef::DeployInterchainToken { token_id, .. }
            | ItsMessageRef::LinkToken { token_id, .. } => token_id,
        }
    }
}

impl<'a> TryFrom<&'a GMPPayload> for ItsMessageRef<'a> {
    type Error = ProgramError;
    fn try_from(value: &'a GMPPayload) -> Result<Self, Self::Error> {
        Ok(match value {
            GMPPayload::InterchainTransfer(inner) => Self::InterchainTransfer {
                token_id: Cow::Borrowed(&inner.token_id.0),
                source_address: &inner.source_address.0,
                destination_address: inner.destination_address.as_ref(),
                amount: inner
                    .amount
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
                data: inner.data.as_ref(),
            },
            GMPPayload::DeployInterchainToken(inner) => Self::DeployInterchainToken {
                token_id: Cow::Borrowed(&inner.token_id.0),
                name: &inner.name,
                symbol: &inner.symbol,
                decimals: inner.decimals,
                minter: inner.minter.as_ref(),
            },
            GMPPayload::LinkToken(inner) => Self::LinkToken {
                token_id: Cow::Borrowed(&inner.token_id.0),
                source_token_address: inner.source_token_address.as_ref(),
                destination_token_address: inner.destination_token_address.as_ref(),
                token_manager_type: inner
                    .token_manager_type
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
                link_params: inner.link_params.as_ref(),
            },
            GMPPayload::RegisterTokenMetadata(_)
            | GMPPayload::SendToHub(_)
            | GMPPayload::ReceiveFromHub(_) => return Err(ProgramError::InvalidArgument),
        })
    }
}

impl TryFrom<RoleManagementInstruction<Roles>> for InterchainTokenServiceInstruction {
    type Error = ProgramError;

    fn try_from(value: RoleManagementInstruction<Roles>) -> Result<Self, Self::Error> {
        match value {
            // Adding and removing operators on the InterchainTokenService is not supported.
            RoleManagementInstruction::AddRoles(_) | RoleManagementInstruction::RemoveRoles(_) => {
                Err(ProgramError::InvalidInstructionData)
            }
            RoleManagementInstruction::TransferRoles(_)
            | RoleManagementInstruction::ProposeRoles(_)
            | RoleManagementInstruction::AcceptRoles(_) => {
                Ok(Self::OperatorInstruction(value.try_into()?))
            }
        }
    }
}

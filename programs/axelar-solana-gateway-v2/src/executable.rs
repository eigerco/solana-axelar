use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::InstructionData;

use crate as axelar_solana_gateway_v2;

use crate::payload;

// Re-export Message
pub use crate::Message;
pub use payload::AxelarMessagePayload as ExecutablePayload;
pub use payload::EncodingScheme as ExecutablePayloadEncodingScheme;

#[error_code]
pub enum ExecutableError {
    #[msg("Payload hash does not match the computed hash of the payload")]
    InvalidPayloadHash,
    #[msg("Provided accounts are invalid")]
    InvalidAccounts,
}

/// Holds references to the Axelar executable accounts needed for validation.
/// This is returned by the `HasAxelarExecutable` trait.
pub struct AxelarExecutableAccountRefs<'a, 'info> {
    pub incoming_message_pda: &'a AccountLoader<'info, axelar_solana_gateway_v2::IncomingMessage>,
    pub signing_pda: &'a AccountInfo<'info>,
    pub gateway_root_pda: &'a AccountLoader<'info, axelar_solana_gateway_v2::state::GatewayConfig>,
    pub axelar_gateway_program:
        &'a Program<'info, axelar_solana_gateway_v2::program::AxelarSolanaGatewayV2>,
    pub event_authority: &'a SystemAccount<'info>,
}

/// Trait that must be implemented by account structs that contain Axelar executable accounts.
/// This trait is automatically implemented when using the `executable_accounts!` macro.
pub trait HasAxelarExecutable<'info> {
    fn axelar_executable(&self) -> AxelarExecutableAccountRefs<'_, 'info>;
}

/// Anchor discriminator for the `execute` instruction.
/// sha256("global:execute")[..8]
///
/// Useful for when the execute instruction has a different name in the program.
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
/// use axelar_solana_gateway_v2::executable::EXECUTE_IX_DISC;
/// use axelar_solana_gateway_v2::Message;
///
/// declare_id!("YourProgramId11111111111111111111111111111");
///
/// #[program]
/// pub mod your_program {
///     use super::*;
///
///     #[instruction(discriminator = EXECUTE_IX_DISC)]
///     pub fn process_gmp(
///         ctx: Context<ProcessGmp>,
///         message: Message,
///         payload: Vec<u8>
///     ) -> Result<()> {
///         // Your GMP message handling logic
///         Ok(())
///     }
/// }
/// ```
pub const EXECUTE_IX_DISC: &[u8; 8] = &[130, 221, 242, 154, 13, 193, 189, 29];

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const EXECUTE_PROGRAM_ACCOUNTS_START_INDEX: usize = 5;

/// Macro to generate executable accounts and validation function.
/// Usage:
/// ```ignore
/// use anchor_lang::prelude::*;
/// use axelar_solana_gateway_v2::{executable::*, executable_accounts};
///
/// executable_accounts!(Execute);
///
/// #[derive(Accounts)]
/// pub struct Execute<'info> {
///     // GMP Accounts
///     pub executable: AxelarExecuteAccounts<'info>,
///
///     // Your program accounts here
/// }
///
/// pub fn execute_handler(ctx: Context<Execute>, message: Message, payload: Vec<u8>) -> Result<()> {
///     validate_message(&ctx.accounts, message, &payload)?;
///
///     Ok(())
/// }
/// ```
///
/// NOTE: Keep in mind the outer accounts struct must not include:
/// ```ignore
/// #[instruction(message: Message, payload: Vec<u8>)]
/// ```
/// attribute due to [a bug](https://github.com/solana-foundation/anchor/issues/2942) in Anchor.
// NOTE: This macro is necessary because Anchor currently does not support importing
// accounts from other crates. Once Anchor supports this, we can remove this macro and
// export the accounts directly from axelar-solana-gateway-v2.
// See: https://github.com/solana-foundation/anchor/issues/3811
// It is also not possible to use the `cpi` module inside the gateway crate.
#[macro_export]
macro_rules! executable_accounts {
    ($outer_struct:ident) => {
    /// Accounts for executing an inbound Axelar GMP message.
    /// NOTE: Keep in mind the outer accounts struct must not include:
    /// ```ignore
    /// #[instruction(message: Message, payload: Vec<u8>)]
    /// ```
    /// attribute due to [a bug](https://github.com/solana-foundation/anchor/issues/2942) in Anchor.
    #[derive(Accounts)]
    #[instruction(message: axelar_solana_gateway_v2::Message)]
    pub struct AxelarExecuteAccounts<'info> {
        // IncomingMessage PDA account
        // needs to be mutable as the validate_message CPI
        // updates its state
        #[account(
        	mut,
            seeds = [axelar_solana_gateway_v2::IncomingMessage::SEED_PREFIX, message.command_id().as_ref()],
            bump = incoming_message_pda.load()?.bump,
            seeds::program = axelar_gateway_program.key()
        )]
        pub incoming_message_pda: AccountLoader<'info, axelar_solana_gateway_v2::IncomingMessage>,

        #[account(
            seeds = [axelar_solana_gateway_v2::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED, message.command_id().as_ref()],
            bump = incoming_message_pda.load()?.signing_pda_bump,
        )]
        pub signing_pda: AccountInfo<'info>,

        #[account(
            seeds = [axelar_solana_gateway_v2::state::GatewayConfig::SEED_PREFIX],
            bump = gateway_root_pda.load()?.bump,
            seeds::program = axelar_gateway_program.key(),
        )]
        pub gateway_root_pda: AccountLoader<'info, axelar_solana_gateway_v2::state::GatewayConfig>,

        #[account(
            seeds = [b"__event_authority"],
            bump,
            seeds::program = axelar_gateway_program.key()
        )]
        pub event_authority: SystemAccount<'info>,

        pub axelar_gateway_program:
            Program<'info, axelar_solana_gateway_v2::program::AxelarSolanaGatewayV2>,
    }

    impl<'info> axelar_solana_gateway_v2::executable::HasAxelarExecutable<'info> for $outer_struct<'info> {
        fn axelar_executable(&self) -> axelar_solana_gateway_v2::executable::AxelarExecutableAccountRefs<'_, 'info> {
            (&self.executable).into()
        }
    }

    impl<'a, 'info> From<&'a AxelarExecuteAccounts<'info>> for axelar_solana_gateway_v2::executable::AxelarExecutableAccountRefs<'a, 'info> {
        fn from(accounts: &'a AxelarExecuteAccounts<'info>) -> Self {
            Self {
                incoming_message_pda: &accounts.incoming_message_pda,
                signing_pda: &accounts.signing_pda,
                gateway_root_pda: &accounts.gateway_root_pda,
                event_authority: &accounts.event_authority,
                axelar_gateway_program: &accounts.axelar_gateway_program,
            }
        }
    }

    };
}

/// Validates an Axelar message with automatic payload reconstruction and account verification.
///
/// Reconstructs the full payload from the payload bytes and account metadata, verifies
/// the accounts match those provided in the instruction, then validates the message hash
/// via CPI to the Axelar gateway.
///
/// # Example
///
/// ```ignore
/// validate_message(
///     &ctx.accounts,
///     message,
///     &payload_bytes,
///     EncodingScheme::Borsh
/// )?;
/// ```
///
/// # Notes
///
/// This is the recommended validation function for standard Axelar GMP messages that
/// include account metadata. For pre-encoded payloads, use `validate_message_raw` instead.
pub fn validate_message<'info, T: HasAxelarExecutable<'info> + ToAccountMetas>(
    accounts: &T,
    message: axelar_solana_gateway_v2::Message,
    payload_without_accounts: &[u8],
    encoding_scheme: axelar_solana_gateway_v2::executable::ExecutablePayloadEncodingScheme,
) -> Result<()> {
    let instruction_accounts = accounts
        .to_account_metas(None)
        .split_off(EXECUTE_PROGRAM_ACCOUNTS_START_INDEX);

    // Reconstruct the ExecutablePayload from the passed accounts
    // and the payload passed in instruction data

    let payload = axelar_solana_gateway_v2::executable::ExecutablePayload::new(
        payload_without_accounts,
        &instruction_accounts,
        encoding_scheme,
    );

    // Verify that the payload hash matches the computed hash of the payload
    let encoded = payload.encode()?;

    let executable_accounts = accounts.axelar_executable();
    validate_message_raw(&executable_accounts, message, &encoded)?;

    Ok(())
}

/// Validates a raw message payload against the Axelar gateway without account reconstruction.
///
/// This is a lower-level validation function that verifies the payload hash matches
/// the expected hash in the message, then performs a CPI call to the Axelar gateway
/// to mark the message as executed.
///
/// # Example
///
/// ```ignore
/// // Validate with pre-encoded payload
/// let encoded_payload = my_payload.encode()?;
/// validate_message_raw(&ctx.accounts.executable, message, &encoded_payload)?;
/// ```
///
/// # Notes
///
/// Unlike `validate_message`, this function:
/// - Does not reconstruct account metadata from the payload
/// - Does not verify payload encoding schemes
/// - Requires the caller to provide the final encoded payload bytes
///
/// Use this function when you have already encoded your payload or when you're
/// not using the standard Axelar payload encoding with account metadata.
pub fn validate_message_raw<'info>(
    executable_accounts: &AxelarExecutableAccountRefs<'_, 'info>,
    message: axelar_solana_gateway_v2::Message,
    payload: &[u8],
) -> Result<()> {
    let computed_payload_hash = anchor_lang::solana_program::keccak::hash(payload).to_bytes();
    if computed_payload_hash != message.payload_hash {
        return err!(axelar_solana_gateway_v2::executable::ExecutableError::InvalidPayloadHash);
    }

    // Prepare signer seeds
    let command_id = message.command_id();
    let signer_seeds = &[
        axelar_solana_gateway_v2::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED,
        &command_id,
        &[executable_accounts
            .incoming_message_pda
            .load()?
            .signing_pda_bump],
    ];
    let signer_seeds = &[&signer_seeds[..]];

    // Prepare CPI accounts

    let cpi_accounts =
        axelar_solana_gateway_v2::__cpi_client_accounts_validate_message::ValidateMessage {
            incoming_message_pda: executable_accounts.incoming_message_pda.to_account_info(),
            caller: executable_accounts.signing_pda.to_account_info(),
            gateway_root_pda: executable_accounts.gateway_root_pda.to_account_info(),
            event_authority: executable_accounts.event_authority.to_account_info(),
            program: executable_accounts.axelar_gateway_program.to_account_info(),
        };

    // Prepare instruction

    // Build the instruction manually
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: executable_accounts.axelar_gateway_program.key(),
        accounts: cpi_accounts.to_account_metas(None),
        data: axelar_solana_gateway_v2::instruction::ValidateMessage { message }.data(),
    };

    let ix_accounts = cpi_accounts.to_account_infos();

    // Call the validate_message CPI
    invoke_signed(&ix, &ix_accounts, signer_seeds)?;

    Ok(())
}

/// Relayer helpers for building the execute instruction
/// for arbitrary programs.
// TODO verify this is the best API for relayers and add tests
pub mod helpers {
    use super::*;
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

    //
    // Instruction
    //

    #[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
    pub struct AxelarExecuteInstruction {
        pub message: Message,
        pub payload_without_accounts: Vec<u8>,
        pub encoding_scheme: ExecutablePayloadEncodingScheme,
    }

    impl anchor_lang::Discriminator for AxelarExecuteInstruction {
        const DISCRIMINATOR: &'static [u8] = EXECUTE_IX_DISC;
    }

    // Use default implementation
    impl InstructionData for AxelarExecuteInstruction {}

    //
    // Accounts
    //

    /// Generated client accounts for [`AxelarExecuteAccounts`].
    // TODO impl AccountInfos and AccountMetas
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AxelarExecuteAccounts {
        pub incoming_message_pda: Pubkey,
        pub signing_pda: Pubkey,
        pub gateway_root_pda: Pubkey,
        pub event_authority: Pubkey,
        pub axelar_gateway_program: Pubkey,
    }

    impl anchor_lang::ToAccountMetas for AxelarExecuteAccounts {
        fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
            vec![
                AccountMeta::new(self.incoming_message_pda, false),
                AccountMeta::new_readonly(self.signing_pda, false),
                AccountMeta::new_readonly(self.gateway_root_pda, false),
                AccountMeta::new_readonly(self.event_authority, false),
                AccountMeta::new_readonly(self.axelar_gateway_program, false),
            ]
        }
    }

    //
    // Instruction builder
    //

    /// Creates an `AxelarExecuteInstruction` for the given parameters.
    pub fn create_execute_instruction(
        program_id: Pubkey,
        message: Message,
        payload: &ExecutablePayload,
        execute_accounts: &AxelarExecuteAccounts,
    ) -> Instruction {
        let ix_data = AxelarExecuteInstruction {
            message,
            payload_without_accounts: payload.payload_without_accounts().to_owned(),
            encoding_scheme: payload.encoding_scheme(),
        }
        .data();

        let accounts = {
            let mut executable = execute_accounts.to_account_metas(None);
            executable.extend(payload.account_meta());
            executable
        };

        Instruction {
            program_id,
            accounts,
            data: ix_data,
        }
    }
}

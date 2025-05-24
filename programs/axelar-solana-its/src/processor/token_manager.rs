//! Processor for [`TokenManager`] related requests.

use event_utils::Event as _;
use program_utils::{BorshPda, ValidPDA};
use role_management::processor::ensure_roles;
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::program_option::COption;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program, sysvar};
use spl_token_2022::check_spl_token_program_account;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions};
use spl_token_2022::instruction::AuthorityType;
use spl_token_2022::state::Mint;

use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, event};
use crate::{assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice, Roles};

pub(crate) fn set_flow_limit(
    accounts: &SetFlowLimitAccounts<'_>,
    flow_limit: u64,
) -> ProgramResult {
    ensure_roles(
        &crate::id(),
        accounts.token_manager_pda,
        accounts.flow_limiter,
        accounts.token_manager_user_roles_pda,
        Roles::FLOW_LIMITER,
    )?;

    let mut token_manager = TokenManager::load(accounts.token_manager_pda)?;
    token_manager.flow_limit = flow_limit;
    token_manager.store(
        accounts.flow_limiter,
        accounts.token_manager_pda,
        accounts.system_account,
    )?;

    Ok(())
}

pub(crate) struct DeployTokenManagerInternal {
    manager_type: token_manager::Type,
    token_id: [u8; 32],
    token_address: Pubkey,
    operator: Option<Pubkey>,
    minter: Option<Pubkey>,
}

impl DeployTokenManagerInternal {
    pub(crate) const fn new(
        manager_type: token_manager::Type,
        token_id: [u8; 32],
        token_address: Pubkey,
        operator: Option<Pubkey>,
        minter: Option<Pubkey>,
    ) -> Self {
        Self {
            manager_type,
            token_id,
            token_address,
            operator,
            minter,
        }
    }
}

/// Deploys a new [`TokenManager`] PDA.
///
/// # Errors
///
/// An error occurred when deploying the [`TokenManager`] PDA. The reason can be
/// derived from the logs.
pub(crate) fn deploy<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &DeployTokenManagerAccounts<'a>,
    deploy_token_manager: &DeployTokenManagerInternal,
    token_manager_pda_bump: u8,
) -> ProgramResult {
    msg!("Instruction: TM Deploy");
    check_accounts(accounts)?;

    validate_token_manager_type(
        deploy_token_manager.manager_type,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    crate::create_associated_token_account(
        payer,
        accounts.token_mint,
        accounts.token_manager_ata,
        accounts.token_manager_pda,
        accounts.system_account,
        accounts.token_program,
    )?;

    if let Some(operator_from_message) = deploy_token_manager.operator {
        let (Some(operator), Some(operator_roles_pda)) =
            (accounts.operator, accounts.operator_roles_pda)
        else {
            return Err(ProgramError::InvalidArgument);
        };

        if operator_from_message.ne(operator.key) {
            msg!("Invalid operator provided");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut roles = Roles::OPERATOR | Roles::FLOW_LIMITER;
        if deploy_token_manager.minter.is_some()
            && deploy_token_manager.manager_type == token_manager::Type::NativeInterchainToken
        {
            roles |= Roles::MINTER;
        }

        setup_roles(
            payer,
            accounts.token_manager_pda,
            operator.key,
            operator_roles_pda,
            accounts.system_account,
            roles,
        )?;
    }

    setup_roles(
        payer,
        accounts.token_manager_pda,
        accounts.its_root_pda.key,
        accounts.its_roles_pda,
        accounts.system_account,
        Roles::OPERATOR | Roles::FLOW_LIMITER,
    )?;

    let token_manager = TokenManager::new(
        deploy_token_manager.manager_type,
        deploy_token_manager.token_id,
        deploy_token_manager.token_address,
        *accounts.token_manager_ata.key,
        token_manager_pda_bump,
    );
    token_manager.init(
        &crate::id(),
        accounts.system_account,
        payer,
        accounts.token_manager_pda,
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root_pda.key.as_ref(),
            &token_manager.token_id,
            &[token_manager.bump],
        ],
    )?;

    event::TokenManagerDeployed {
        token_id: deploy_token_manager.token_id,
        token_manager: *accounts.token_manager_pda.key,
        token_manager_type: deploy_token_manager.manager_type.into(),
        params: deploy_token_manager
            .operator
            .map(|op| op.to_bytes().to_vec())
            .unwrap_or_default(),
    }
    .emit();

    Ok(())
}

fn setup_roles<'a>(
    payer: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    user: &Pubkey,
    user_roles_pda: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    roles: Roles,
) -> ProgramResult {
    let (derived_user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), token_manager_pda.key, user);

    if derived_user_roles_pda.ne(user_roles_pda.key) {
        msg!("Invalid user roles PDA provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if let Ok(mut existing_roles) = UserRoles::<Roles>::load(user_roles_pda) {
        existing_roles.add(roles);
        existing_roles.store(payer, user_roles_pda, system_account)?;
    } else {
        let user_roles = UserRoles::new(roles, user_roles_pda_bump);
        user_roles.init(
            &crate::id(),
            system_account,
            payer,
            user_roles_pda,
            &[
                role_management::seed_prefixes::USER_ROLES_SEED,
                token_manager_pda.key.as_ref(),
                user.as_ref(),
                &[user_roles_pda_bump],
            ],
        )?;
    }

    Ok(())
}

fn check_accounts(accounts: &DeployTokenManagerAccounts<'_>) -> ProgramResult {
    if !system_program::check_id(accounts.system_account.key) {
        msg!("Invalid system account provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    if accounts
        .token_manager_pda
        .check_uninitialized_pda()
        .is_err()
    {
        msg!("TokenManager PDA is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if spl_token_2022::check_spl_token_program_account(accounts.token_mint.owner).is_err() {
        msg!("Invalid token mint account provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if accounts.token_program.key != accounts.token_mint.owner {
        msg!("Mint and program account mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_associated_token_account::check_id(accounts.ata_program.key) {
        msg!("Invalid associated token account program provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub(crate) fn validate_token_manager_type(
    ty: token_manager::Type,
    token_mint: &AccountInfo<'_>,
    token_manager_pda: &AccountInfo<'_>,
) -> ProgramResult {
    let mint_data = token_mint.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    match (mint.base.mint_authority, ty) {
        (COption::None, token_manager::Type::NativeInterchainToken) => {
            msg!("Mint authority is required for native InterchainToken");
            Err(ProgramError::InvalidInstructionData)
        }
        (COption::Some(key), token_manager::Type::NativeInterchainToken)
            if &key != token_manager_pda.key =>
        {
            msg!("TokenManager is not the mint authority, which is required for this token manager type");
            Err(ProgramError::InvalidInstructionData)
        }
        (_, token_manager::Type::LockUnlockFee)
            if !mint
                .get_extension_types()?
                .contains(&ExtensionType::TransferFeeConfig) =>
        {
            msg!("The mint is not compatible with the LockUnlockFee TokenManager type, please make sure the mint has the TransferFeeConfig extension initialized");
            Err(ProgramError::InvalidAccountData)
        }
        _ => Ok(()),
    }
}

pub(crate) fn handover_mint_authority(
    accounts: &[AccountInfo<'_>],
    token_id: [u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let its_root = next_account_info(accounts_iter)?;
    let token_manager = next_account_info(accounts_iter)?;
    let minter_roles = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    msg!("Instruction: TM Hand Over Mint Authority");
    let its_root_config = InterchainTokenService::load(its_root)?;
    let token_manager_config = TokenManager::load(token_manager)?;

    assert_valid_its_root_pda(its_root, its_root_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager,
        its_root.key,
        &token_id,
        token_manager_config.bump,
    )?;

    let mint_authority = {
        let mint_data = mint.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

        mint.base.mint_authority
    };

    match mint_authority {
        COption::None => {
            msg!("Cannot hand over mint authority of a TokenManager for non-mintable token");
            Err(ProgramError::InvalidArgument)
        }
        COption::Some(authority) if authority == *payer.key => {
            // The payer is the mint authority. The mint authority needs to be transferred
            // to the `TokenManager` and the `minter` role is added to the payer
            // on the `TokenManager`. Future minting by the user needs to go
            // through ITS.
            let authority_transfer_ix = spl_token_2022::instruction::set_authority(
                token_program.key,
                mint.key,
                Some(token_manager.key),
                AuthorityType::MintTokens,
                payer.key,
                &[],
            )?;

            invoke(&authority_transfer_ix, &[mint.clone(), payer.clone()])?;

            setup_roles(
                payer,
                token_manager,
                payer.key,
                minter_roles,
                system_account,
                Roles::MINTER,
            )?;

            Ok(())
        }
        COption::Some(_) => {
            msg!("Signer is not the mint authority");
            Err(ProgramError::InvalidArgument)
        }
    }?;

    Ok(())
}

#[derive(Debug)]
pub(crate) struct DeployTokenManagerAccounts<'a> {
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) its_roles_pda: &'a AccountInfo<'a>,
    pub(crate) _rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) operator: Option<&'a AccountInfo<'a>>,
    pub(crate) operator_roles_pda: Option<&'a AccountInfo<'a>>,
}

impl<'a> FromAccountInfoSlice<'a> for DeployTokenManagerAccounts<'a> {
    type Context = ();

    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized,
    {
        let accounts_iter = &mut accounts.iter();
        let system_account = next_account_info(accounts_iter)?;
        let its_root_pda = next_account_info(accounts_iter)?;
        let token_manager_pda = next_account_info(accounts_iter)?;
        let token_mint = next_account_info(accounts_iter)?;
        let token_manager_ata = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let ata_program = next_account_info(accounts_iter)?;
        let its_roles_pda = next_account_info(accounts_iter)?;
        let rent_sysvar = next_account_info(accounts_iter)?;

        if !system_program::check_id(system_account.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        check_spl_token_program_account(token_program.key)?;
        if !spl_associated_token_account::check_id(ata_program.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        if !sysvar::rent::check_id(rent_sysvar.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            system_account,
            its_root_pda,
            token_manager_pda,
            token_mint,
            token_manager_ata,
            token_program,
            ata_program,
            its_roles_pda,
            _rent_sysvar: rent_sysvar,
            operator: next_account_info(accounts_iter).ok(),
            operator_roles_pda: next_account_info(accounts_iter).ok(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct SetFlowLimitAccounts<'a> {
    pub(crate) flow_limiter: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) its_user_roles_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_user_roles_pda: &'a AccountInfo<'a>,
    pub(crate) system_account: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for SetFlowLimitAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error> {
        let accounts_iter = &mut value.iter();
        let flow_limiter = next_account_info(accounts_iter)?;
        let its_root_pda = next_account_info(accounts_iter)?;
        let token_manager_pda = next_account_info(accounts_iter)?;
        let its_user_roles_pda = next_account_info(accounts_iter)?;
        let token_manager_user_roles_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;
        if !system_program::check_id(system_account.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            flow_limiter,
            its_root_pda,
            token_manager_pda,
            its_user_roles_pda,
            token_manager_user_roles_pda,
            system_account,
        })
    }
}

#[cfg(test)]
mod tests {
    use solana_program::account_info::AccountInfo;
    use solana_program::program_error::ProgramError;
    use solana_program::pubkey::Pubkey;
    use solana_program::{system_program, sysvar};

    use crate::processor::token_manager::{DeployTokenManagerAccounts, SetFlowLimitAccounts};
    use crate::FromAccountInfoSlice;

    use super::handover_mint_authority;

    struct TestAccount<'a> {
        _pubkey: Box<Pubkey>,
        _lamports: Box<u64>,
        _data: Box<[u8; 3]>,
        info: AccountInfo<'a>,
    }

    impl<'a> TestAccount<'a> {
        fn new(pubkey: Pubkey) -> Self {
            let pubkey = Box::new(pubkey);
            let lamports = Box::new(2);
            let data = Box::new([1, 2, 3]);

            let info = AccountInfo::new(
                Box::leak(pubkey.clone()),
                true,
                true,
                Box::leak(lamports.clone()),
                Box::leak(data.clone()),
                Box::leak(pubkey.clone()),
                true,
                1,
            );

            Self {
                _pubkey: pubkey,
                _lamports: lamports,
                _data: data,
                info,
            }
        }

        fn with_custom(pubkey: Pubkey, lamports_val: u64) -> Self {
            let pubkey = Box::new(pubkey);
            let lamports = Box::new(lamports_val);
            let data = Box::new([1, 2, 3]);

            let info = AccountInfo::new(
                Box::leak(pubkey.clone()),
                true,
                true,
                Box::leak(lamports.clone()),
                Box::leak(data.clone()),
                Box::leak(pubkey.clone()),
                true,
                1,
            );

            Self {
                _pubkey: pubkey,
                _lamports: lamports,
                _data: data,
                info,
            }
        }
    }

    struct TestAccounts<'a> {
        accounts: Vec<AccountInfo<'a>>,
        dummy: TestAccount<'a>,
    }

    fn get_valid_accounts<'a>() -> TestAccounts<'a> {
        let dummy_key = Pubkey::new_unique();

        let system = TestAccount::with_custom(system_program::ID, 1);
        let dummy = TestAccount::new(dummy_key);
        let token = TestAccount::new(spl_token_2022::ID);
        let ata = TestAccount::new(spl_associated_token_account::ID);
        let rent = TestAccount::new(sysvar::rent::ID);

        let accounts = vec![
            system.info.clone(),
            dummy.info.clone(),
            dummy.info.clone(),
            dummy.info.clone(),
            dummy.info.clone(),
            token.info.clone(),
            ata.info.clone(),
            dummy.info.clone(),
            rent.info.clone(),
        ];

        TestAccounts { accounts, dummy }
    }

    #[allow(clippy::indexing_slicing)]
    fn test_invalid_index(index: usize) {
        let mut test = get_valid_accounts();
        test.accounts[index] = test.dummy.info.clone();

        let result = DeployTokenManagerAccounts::from_account_info_slice(&test.accounts, &());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::IncorrectProgramId);
    }

    #[test]
    fn test_valid_accounts() {
        let test = get_valid_accounts();
        let result = DeployTokenManagerAccounts::from_account_info_slice(&test.accounts, &());
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_system_account() {
        test_invalid_index(0);
    }

    #[test]
    fn test_invalid_token_program() {
        test_invalid_index(5);
    }

    #[test]
    fn test_invalid_program_ata() {
        test_invalid_index(6);
    }

    #[test]
    fn test_invalid_rent() {
        test_invalid_index(8);
    }

    #[test]
    fn test_accounts_for_set_flow_limit_accounts() {
        let key = Pubkey::new_unique();

        let mut system_account_lamports = 1;
        let mut system_account_data = [1, 2, 3];
        let system_account = AccountInfo::new(
            &system_program::ID,
            true,
            true,
            &mut system_account_lamports,
            &mut system_account_data,
            &key,
            true,
            1,
        );

        let mut lamports = 2;
        let mut data = [1, 2, 3];
        let dummy_account =
            AccountInfo::new(&key, true, true, &mut lamports, &mut data, &key, true, 1);

        let accounts = [
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            system_account,
        ];
        let parsed_accounts = SetFlowLimitAccounts::try_from(accounts.as_slice());
        assert!(parsed_accounts.is_ok());

        // Switch system account to make it fail
        let accounts = [
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account,
        ];
        let parsed_accounts = SetFlowLimitAccounts::try_from(accounts.as_slice());
        assert!(parsed_accounts.is_err());
        assert_eq!(
            parsed_accounts.unwrap_err(),
            ProgramError::IncorrectProgramId
        );
    }

    #[test]
    fn test_handover_mint_authority() {
        let key = Pubkey::new_unique();

        let mut system_account_lamports = 1;
        let mut system_account_data = [1, 2, 3];
        let system_account = AccountInfo::new(
            &system_program::ID,
            true,
            true,
            &mut system_account_lamports,
            &mut system_account_data,
            &key,
            true,
            1,
        );

        let mut lamports = 2;
        let mut data = [1, 2, 3];
        let dummy_account =
            AccountInfo::new(&key, true, true, &mut lamports, &mut data, &key, true, 1);

        let mut accounts = [
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
            dummy_account.clone(),
        ];

        let res = handover_mint_authority(&accounts, [0; 32]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ProgramError::IncorrectProgramId);

        // Change value to system account
        // Fails in different place, but not on parsing accounts
        accounts[6] = system_account;
        let res = handover_mint_authority(&accounts, [0; 32]);
        assert!(res.is_err());
        assert_ne!(res.unwrap_err(), ProgramError::IncorrectProgramId);
    }
}

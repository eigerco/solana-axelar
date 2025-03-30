// This file is autogenerated with https://github.com/acheroncrypto/native-to-anchor

use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod axelar_solana_its {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        chain_name: String,
        its_hub_address: String,
    ) -> Result<()> {
        Ok(())
    }

    pub fn set_pause_status(ctx: Context<SetPauseStatus>, paused: bool) -> Result<()> {
        Ok(())
    }

    pub fn set_trusted_chain(ctx: Context<SetTrustedChain>, chain_name: String) -> Result<()> {
        Ok(())
    }

    pub fn remove_trusted_chain(
        ctx: Context<RemoveTrustedChain>,
        chain_name: String,
    ) -> Result<()> {
        Ok(())
    }

    pub fn approve_deploy_remote_interchain_token(
        ctx: Context<ApproveDeployRemoteInterchainToken>,
        deployer: Pubkey,
        salt: [u8; 32],
        destination_chain: String,
        destination_minter: Vec<u8>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn revoke_deploy_remote_interchain_token(
        ctx: Context<RevokeDeployRemoteInterchainToken>,
        deployer: Pubkey,
        salt: [u8; 32],
        destination_chain: String,
    ) -> Result<()> {
        Ok(())
    }

    pub fn register_canonical_interchain_token(
        ctx: Context<RegisterCanonicalInterchainToken>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn deploy_remote_canonical_interchain_token(
        ctx: Context<DeployRemoteCanonicalInterchainToken>,
        destination_chain: String,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn interchain_transfer(
        ctx: Context<InterchainTransfer>,
        token_id: [u8; 32],
        destination_chain: String,
        destination_address: Vec<u8>,
        amount: u64,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn deploy_interchain_token(
        ctx: Context<DeployInterchainToken>,
        salt: [u8; 32],
        name: String,
        symbol: String,
        decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn deploy_remote_interchain_token(
        ctx: Context<DeployRemoteInterchainToken>,
        salt: [u8; 32],
        destination_chain: String,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn deploy_remote_interchain_token_with_minter(
        ctx: Context<DeployRemoteInterchainTokenWithMinter>,
        salt: [u8; 32],
        destination_chain: String,
        destination_minter: Vec<u8>,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn register_token_metadata(
        ctx: Context<RegisterTokenMetadata>,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn register_custom_token(
        ctx: Context<RegisterCustomToken>,
        salt: [u8; 32],
        token_manager_type: Type,
        operator: Option<Pubkey>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn link_token(
        ctx: Context<LinkToken>,
        salt: [u8; 32],
        destination_chain: String,
        destination_token_address: Vec<u8>,
        token_manager_type: Type,
        link_params: Vec<u8>,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn call_contract_with_interchain_token(
        ctx: Context<CallContractWithInterchainToken>,
        token_id: [u8; 32],
        destination_chain: String,
        destination_address: Vec<u8>,
        amount: u64,
        data: Vec<u8>,
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn call_contract_with_interchain_token_offchain_data(
        ctx: Context<CallContractWithInterchainTokenOffchainData>,
        token_id: [u8; 32],
        destination_chain: String,
        destination_address: Vec<u8>,
        amount: u64,
        payload_hash: [u8; 32],
        gas_value: u64,
        signing_pda_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn set_flow_limit(ctx: Context<SetFlowLimit>, flow_limit: u64) -> Result<()> {
        Ok(())
    }

    pub fn operator_transfer_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn operator_propose_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn operator_accept_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_add_flow_limiter(
        ctx: Context<TokenManagerFlowLimiter>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_remove_flow_limiter(
        ctx: Context<TokenManagerFlowLimiter>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_set_flow_limit(
        ctx: Context<TokenManagerSetFlowLimit>,
        flow_limit: u64,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_transfer_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_propose_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_accept_operatorship(
        ctx: Context<Operator>,
        inputs: RoleManagementInstructionInputs,
    ) -> Result<()> {
        Ok(())
    }

    pub fn token_manager_hand_over_mint_authority(
        ctx: Context<TokenManagerHandOverMintAuthority>,
        token_id: [u8; 32],
    ) -> Result<()> {
        Ok(())
    }

    pub fn interchain_token_mint(
        ctx: Context<InterchainTokenMint>,
        amount: u64,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    program_data_address: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    #[account(mut)]
    its_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
    operator: AccountInfo<'info>,
    #[account(mut)]
    user_roles_pda: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetPauseStatus<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    program_data_address: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    #[account(mut)]
    its_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetTrustedChain<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    program_data_address: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    #[account(mut)]
    its_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveTrustedChain<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    program_data_address: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    #[account(mut)]
    its_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveDeployRemoteInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    token_manager_pda: AccountInfo<'info>,
    roles_pda: AccountInfo<'info>,
    #[account(mut)]
    deploy_approval_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevokeDeployRemoteInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    deploy_approval_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterCanonicalInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    token_metadata_account: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    spl_associated_token_account: AccountInfo<'info>,
    #[account(mut)]
    its_user_roles_pda: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DeployRemoteCanonicalInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    mint: AccountInfo<'info>,
    metadata_account: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    mpl_token_metadata: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InterchainTransfer<'info> {
    payer: Signer<'info>,
    authority: Signer<'info>,
    #[account(mut)]
    source_account: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    #[account(mut)]
    flow_slot_pda: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DeployInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    gateway_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    spl_associated_token_account: AccountInfo<'info>,
    #[account(mut)]
    its_user_roles_pda: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    sysvar_instructions: AccountInfo<'info>,
    mpl_token_metadata: AccountInfo<'info>,
    #[account(mut)]
    metadata_account: AccountInfo<'info>,
    minter: AccountInfo<'info>,
    #[account(mut)]
    minter_roles_pda: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DeployRemoteInterchainToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    mint: AccountInfo<'info>,
    metadata_account: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    mpl_token_metadata: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DeployRemoteInterchainTokenWithMinter<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    mint: AccountInfo<'info>,
    metadata_account: AccountInfo<'info>,
    minter: AccountInfo<'info>,
    #[account(mut)]
    deploy_approval: AccountInfo<'info>,
    minter_roles_pda: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    mpl_token_metadata: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RegisterTokenMetadata<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    mint: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RegisterCustomToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    token_metadata_account: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    spl_associated_token_account: AccountInfo<'info>,
    #[account(mut)]
    its_user_roles_pda: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    #[account(mut)]
    operator: Option<AccountInfo<'info>>,
    #[account(mut)]
    operator_roles_pda: Option<AccountInfo<'info>>,
}

#[derive(Accounts)]
pub struct LinkToken<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    token_manager_pda: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CallContractWithInterchainToken<'info> {
    payer: Signer<'info>,
    authority: Signer<'info>,
    #[account(mut)]
    source_account: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    #[account(mut)]
    flow_slot_pda: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CallContractWithInterchainTokenOffchainData<'info> {
    payer: Signer<'info>,
    authority: Signer<'info>,
    #[account(mut)]
    source_account: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_ata: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    #[account(mut)]
    flow_slot_pda: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    axelar_solana_gateway: AccountInfo<'info>,
    #[account(mut)]
    gas_config_pda: AccountInfo<'info>,
    gas_service: AccountInfo<'info>,
    system_program: Program<'info, System>,
    its_root_pda: AccountInfo<'info>,
    call_contract_signing_pda: AccountInfo<'info>,
    ID: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetFlowLimit<'info> {
    payer: Signer<'info>,
    its_root_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_pda: AccountInfo<'info>,
    its_user_roles_pda: AccountInfo<'info>,
    token_manager_user_roles_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Operator<'info> {
    gateway_root_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
    #[account(mut)]
    payer: Signer<'info>,
    payer_roles_account: AccountInfo<'info>,
    resource: AccountInfo<'info>,
    destination_user_account: AccountInfo<'info>,
    destination_roles_account: AccountInfo<'info>,
    #[account(mut)]
    origin_user_account: AccountInfo<'info>,
    origin_roles_account: AccountInfo<'info>,
    #[account(mut)]
    proposal_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TokenManagerFlowLimiter<'info> {
    system_program: Program<'info, System>,
    #[account(mut)]
    payer: Signer<'info>,
    payer_roles_account: AccountInfo<'info>,
    resource: AccountInfo<'info>,
    destination_user_account: AccountInfo<'info>,
    destination_roles_account: AccountInfo<'info>,
    #[account(mut)]
    origin_user_account: AccountInfo<'info>,
    origin_roles_account: AccountInfo<'info>,
    #[account(mut)]
    proposal_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TokenManagerSetFlowLimit<'info> {
    payer: Signer<'info>,
    its_root_pda: AccountInfo<'info>,
    #[account(mut)]
    token_manager_pda: AccountInfo<'info>,
    token_manager_user_roles_pda: AccountInfo<'info>,
    its_user_roles_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TokenManagerHandOverMintAuthority<'info> {
    payer: Signer<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    gateway_root_pda: AccountInfo<'info>,
    its_root_pda: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    #[account(mut)]
    minter_roles_pda: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InterchainTokenMint<'info> {
    #[account(mut)]
    mint: AccountInfo<'info>,
    #[account(mut)]
    destination_account: AccountInfo<'info>,
    its_root_pda: AccountInfo<'info>,
    token_manager_pda: AccountInfo<'info>,
    minter: AccountInfo<'info>,
    minter_roles_pda: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum Type {
    NativeInterchainToken,
    MintBurnFrom,
    LockUnlock,
    LockUnlockFee,
    MintBurn,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RoleManagementInstructionInputs {
    pub roles: Roles,
    pub destination_roles_pda_bump: u8,
    pub proposal_pda_bump: Option<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum Roles {
    Minter = 1,
    Operator = 2,
    FlowLimiter = 4,
}

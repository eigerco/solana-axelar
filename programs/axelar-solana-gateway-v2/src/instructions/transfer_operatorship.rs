use crate::seed_prefixes::GATEWAY_SEED;
use crate::{GatewayConfig, GatewayError, OperatorshipTransferredEvent};
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_lang::solana_program::bpf_loader_upgradeable;

#[derive(Accounts)]
#[event_cpi]
pub struct TransferOperatorship<'info> {
    #[account(
        mut,
        seeds = [GATEWAY_SEED],
        bump = gateway_root_pda.load()?.bump
    )]
    pub gateway_root_pda: AccountLoader<'info, GatewayConfig>,

    #[account(
    	// CHECK: This is either the current operator or the upgrade authority
		constraint = gateway_root_pda.load()?.operator == *operator_or_upgrade_authority.key
			|| program_data.upgrade_authority_address == Some(*operator_or_upgrade_authority.key)
			@ GatewayError::InvalidOperatorOrAuthorityAccount
    )]
    pub operator_or_upgrade_authority: Signer<'info>,

    #[account(
	    seeds = [crate::ID.as_ref()],
	    bump,
	    seeds::program = bpf_loader_upgradeable::ID,
	)]
    pub program_data: Account<'info, ProgramData>,

    #[account(
    	// CHECK: The new operator must be different
    	constraint = new_operator.key() != gateway_root_pda.load()?.operator.key()
     		@ ProgramError::InvalidInstructionData
    )]
    pub new_operator: UncheckedAccount<'info>,
}

pub fn transfer_operatorship_handler(ctx: Context<TransferOperatorship>) -> Result<()> {
    // Update the operator in the gateway config
    let mut gateway_root_pda = ctx.accounts.gateway_root_pda.load_mut()?;
    gateway_root_pda.operator = *ctx.accounts.new_operator.key;

    emit_cpi!(OperatorshipTransferredEvent {
        new_operator: gateway_root_pda.operator.to_bytes()
    });

    Ok(())
}

#![cfg(test)]
use anchor_lang::InstructionData;
use axelar_solana_gas_service::instructions as v1_instructions;
use axelar_solana_gas_service_v2::instruction;
use solana_sdk::pubkey::Pubkey;

#[test]
fn test_v1_instructions_compatibility() {
    let v1_initialize = borsh::to_vec(&v1_instructions::GasServiceInstruction::Initialize).unwrap();
    let v2_initialize = instruction::Initialize {}.data();
    assert_eq!(v1_initialize, v2_initialize);

    // TODO reenable when v1 gets updated
    // let amount = 1000u64;
    // let decimals = 8u8;
    // let v1_collect_spl_fees =
    //     borsh::to_vec(&v1_instructions::GasServiceInstruction::CollectSplFees { amount, decimals })
    //         .unwrap();
    // let v2_collect_spl_fees = instruction::CollectFees { amount, decimals }.data();
    // assert_eq!(v1_collect_spl_fees, v2_collect_spl_fees);

    // TODO add rest of instructions
}

#[test]
fn test_v1_treasury_compatibility() {
    use axelar_solana_gas_service::state::Config;
    use axelar_solana_gas_service_v2::state::Treasury;

    assert_eq!(
        std::mem::size_of::<Config>(),
        std::mem::size_of::<Treasury>()
    );

    // Make v1
    let conf = Config {
        operator: Pubkey::new_unique(),
        bump: 44,
    };

    // Cast to v2
    let bytes = bytemuck::bytes_of(&conf);
    let treasury: &Treasury = bytemuck::from_bytes(bytes);

    // Verify the bump matches
    assert_eq!(conf.bump, treasury.bump);
}

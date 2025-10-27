use anchor_lang::AccountDeserialize;
use axelar_solana_governance_v2::state::{GovernanceConfig, GovernanceConfigInit};
use axelar_solana_governance_v2_test_fixtures::{initialize_governance, mock_setup_test};

#[test]
fn should_initialize_config() {
    let setup = mock_setup_test();
    let chain_hash = [1u8; 32];
    let address_hash = [2u8; 32];
    let minimum_proposal_eta_delay = 3600;

    let governance_config = GovernanceConfigInit::new(
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay,
        setup.operator.to_bytes(),
    );

    let result = initialize_governance(&setup, governance_config.clone());
    assert!(!result.program_result.is_err());

    let governance_config_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| *pubkey == setup.governance_config)
        .unwrap()
        .1
        .clone();

    let actual_config =
        GovernanceConfig::try_deserialize(&mut governance_config_account.data.as_slice()).unwrap();

    assert_eq!(actual_config.chain_hash, governance_config.chain_hash);
    assert_eq!(
        actual_config.minimum_proposal_eta_delay,
        governance_config.minimum_proposal_eta_delay
    );
    assert_eq!(actual_config.operator, governance_config.operator);
}

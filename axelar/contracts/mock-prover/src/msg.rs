use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{HexBinary, Uint256, Uint64};
use multisig_prover::encoding::{Data, Encoder};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin_address: String,
    pub gateway_address: String,
    // pub multisig_address: String,
    // pub service_registry_address: String,
    // pub voting_verifier_address: String,
    pub destination_chain_id: Uint256,
    // pub signing_threshold: Threshold,
    // pub service_name: String,
    pub chain_name: String,
    // pub worker_set_diff_threshold: u32,
    pub encoder: Encoder,
}

#[cw_serde]
pub enum ExecuteMsg {
    ConstructProof { message_ids: Vec<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetProofResponse)]
    GetProof { multisig_session_id: Uint64 },
}

#[cw_serde]
pub struct GetProofResponse {
    pub multisig_session_id: Uint64,
    pub message_ids: Vec<String>,
    pub data: Data,
    pub status: ProofStatus,
}

#[cw_serde]
pub enum ProofStatus {
    Pending,
    Completed { execute_data: HexBinary }, // encoded data and proof sent to destination gateway
}

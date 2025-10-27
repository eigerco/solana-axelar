use anchor_lang::prelude::*;

#[error_code]
pub enum GovernanceError {
    InvalidUpgradeAuthority,
    InvalidArgument,
    NotOperator,
    InvalidPayloadHash,
    ArithmeticOverflow,
    UnauthorizedChain,
    UnauthorizedAddress,
    InvalidInstructionData,
    ProposalNotReady,
    InvalidTargetProgram,
    TargetAccountNotFound,
    MissingNativeValueReceiver,
    InvalidNativeValue,
    InsufficientFunds,
    UnauthorizedOperator,
    MissingRequiredSignature,
    GovernanceConfigMissing,
}

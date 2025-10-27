use axelar_solana_gateway_v2::ID as GATEWAY_PROGRAM_ID;
use axelar_solana_governance_v2::ID as GOVERNANCE_PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program::ID as SYSTEM_PROGRAM_ID;

pub struct AccountWithData {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,
    pub owner: Pubkey,
}

pub struct GmpContext {
    pub incoming_message: AccountWithData,
    pub governance_config: AccountWithData,
    pub signing_pda: AccountWithData,
    pub gateway_root_pda: AccountWithData,
    pub event_authority_pda: AccountWithData,
    pub event_authority_pda_governance: AccountWithData,
    pub proposal: AccountWithData,
    pub operator_proposal: AccountWithData,
}

impl GmpContext {
    pub fn new() -> Self {
        Self {
            incoming_message: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: GATEWAY_PROGRAM_ID,
            },
            governance_config: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: GOVERNANCE_PROGRAM_ID,
            },
            gateway_root_pda: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
            },
            signing_pda: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
            },
            event_authority_pda: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
            },
            event_authority_pda_governance: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: SYSTEM_PROGRAM_ID,
            },
            proposal: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: Pubkey::default(),
            },
            operator_proposal: AccountWithData {
                pubkey: Pubkey::default(),
                data: vec![],
                owner: Pubkey::default(),
            },
        }
    }

    #[must_use]
    pub fn with_incoming_message(mut self, pubkey: Pubkey, data: Vec<u8>) -> Self {
        self.incoming_message = AccountWithData {
            pubkey,
            data,
            owner: GATEWAY_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_governance_config(mut self, pubkey: Pubkey, data: Vec<u8>) -> Self {
        self.governance_config = AccountWithData {
            pubkey,
            data,
            owner: GOVERNANCE_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_signing_pda(mut self, pubkey: Pubkey) -> Self {
        self.signing_pda = AccountWithData {
            pubkey,
            data: vec![],
            owner: SYSTEM_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_gateway_root_pda(mut self, pubkey: Pubkey, data: Vec<u8>) -> Self {
        self.gateway_root_pda = AccountWithData {
            pubkey,
            data,
            owner: GATEWAY_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_event_authority_pda(mut self, pubkey: Pubkey) -> Self {
        self.event_authority_pda = AccountWithData {
            pubkey,
            data: vec![],
            owner: SYSTEM_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_event_authority_pda_governance(mut self, pubkey: Pubkey) -> Self {
        self.event_authority_pda_governance = AccountWithData {
            pubkey,
            data: vec![],
            owner: SYSTEM_PROGRAM_ID,
        };
        self
    }

    #[must_use]
    pub fn with_proposal(mut self, pubkey: Pubkey, data: Vec<u8>, owner: Pubkey) -> Self {
        self.proposal = AccountWithData {
            pubkey,
            data,
            owner,
        };
        self
    }

    #[must_use]
    pub fn with_operator_proposal(mut self, pubkey: Pubkey, data: Vec<u8>, owner: Pubkey) -> Self {
        self.operator_proposal = AccountWithData {
            pubkey,
            data,
            owner,
        };
        self
    }
}

impl Default for GmpContext {
    fn default() -> Self {
        Self::new()
    }
}

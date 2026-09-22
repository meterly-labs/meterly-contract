use soroban_sdk::{contractevent, Address};

/// Emitted when a caller deposits funds into escrow for a provider.
/// Both `caller` and `provider` are topics for filtering.
#[contractevent]
pub struct Deposited {
    #[topic]
    pub caller: Address,
    #[topic]
    pub provider: Address,
    pub amount: i128,
}

/// Emitted when a caller withdraws unused funds from escrow.
/// Both `caller` and `provider` are topics for filtering.
#[contractevent]
pub struct Withdrawn {
    #[topic]
    pub caller: Address,
    #[topic]
    pub provider: Address,
    pub amount: i128,
}

/// Emitted when a settlement is executed, transferring funds from caller's balance to provider.
/// Both `caller` and `provider` are topics for filtering.
#[contractevent]
pub struct Settled {
    #[topic]
    pub caller: Address,
    #[topic]
    pub provider: Address,
    pub amount: i128,
}

/// Emitted when the receipt contract is wired post-deploy.
#[contractevent]
pub struct ReceiptContractSet {
    pub receipt: Address,
}

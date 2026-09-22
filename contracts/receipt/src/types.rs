use soroban_sdk::contracttype;
use soroban_sdk::{Address, BytesN};

/// Storage keys for the Receipt contract.
#[contracttype]
pub enum DataKey {
    /// The escrow contract address (instance storage).
    EscrowAddress,
    /// The registry contract address (instance storage).
    RegistryAddress,
    /// Per-caller nonce for replay protection: Address -> u64
    Nonce(Address),
    /// Stored receipt record: BytesN<32> (receipt_hash) -> ReceiptRecord
    Receipt(BytesN<32>),
}

/// On-chain record of an API call receipt and settlement.
#[contracttype]
#[derive(Clone)]
pub struct ReceiptRecord {
    /// The caller who made the API call.
    pub caller: Address,
    /// The provider who handled the call.
    pub provider: Address,
    /// The amount settled for this call.
    pub amount: i128,
    /// Caller-specific nonce for replay prevention.
    pub nonce: u64,
    /// Ledger timestamp when the receipt was submitted.
    pub timestamp: u64,
}

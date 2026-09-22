use soroban_sdk::{contractevent, Address, BytesN};

/// Emitted when a receipt is successfully submitted and settled.
/// All three fields are topics so the off-chain indexer can filter by:
/// - caller: get all receipts for a specific caller
/// - provider: get all receipts for a specific provider
/// - receipt_hash: get a specific receipt
#[contractevent]
pub struct ReceiptSubmitted {
    #[topic]
    pub caller: Address,
    #[topic]
    pub provider: Address,
    #[topic]
    pub receipt_hash: BytesN<32>,
    pub amount: i128,
}

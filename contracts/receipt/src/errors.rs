use soroban_sdk::contracterror;

/// Errors returned by the Receipt contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called when already initialized.
    AlreadyInitialized = 1,
    /// The nonce submitted is not strictly greater than the caller's last recorded nonce.
    InvalidNonce = 2,
    /// The settlement amount exceeds the provider's price-per-call.
    PriceMismatch = 3,
    /// A receipt with this hash already exists (duplicate submission or hash collision).
    ReceiptExists = 4,
    /// The requested receipt_hash does not exist in storage.
    ReceiptNotFound = 5,
}

use soroban_sdk::contracterror;

/// Errors returned by the Escrow contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called when it was already initialized.
    AlreadyInitialized = 1,
    /// Caller has insufficient balance to withdraw or settle the requested amount.
    InsufficientBalance = 2,
    /// A deposit or settlement amount was zero or negative.
    InvalidAmount = 3,
    /// The caller attempting `settle` is not the authorized receipt contract.
    Unauthorized = 4,
    /// The provider referenced in deposit/withdraw/settle is not registered in the registry.
    ProviderNotFound = 5,
}

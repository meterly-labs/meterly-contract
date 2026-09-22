use soroban_sdk::contracterror;

/// Errors returned by the Registry contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called when an admin is already stored.
    AlreadyInitialized = 1,
    /// The requested provider address has no registration in storage.
    ProviderNotFound = 2,
    /// A price or new_price argument was zero or negative.
    InvalidPrice = 3,
}

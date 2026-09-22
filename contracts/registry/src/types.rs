use soroban_sdk::contracttype;
use soroban_sdk::{Address, String};

/// Storage keys for the Registry contract.
#[contracttype]
pub enum DataKey {
    /// The admin address (instance storage).
    Admin,
    /// Per-provider metadata (persistent storage).
    Provider(Address),
}

/// On-chain record for a registered API provider.
#[contracttype]
#[derive(Clone)]
pub struct ProviderInfo {
    /// The address that registered and controls this provider entry.
    pub owner: Address,
    /// Price per API call in the token's smallest unit (USDC = 7 decimals on Stellar).
    pub price_per_call: i128,
    /// Whether this provider is currently accepting calls.
    pub active: bool,
    /// URI pointing to off-chain JSON metadata (display name, endpoint, docs link).
    pub metadata_uri: String,
}

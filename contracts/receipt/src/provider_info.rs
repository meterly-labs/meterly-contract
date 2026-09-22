use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone)]
pub struct ProviderInfo {
    pub owner: Address,
    pub price_per_call: i128,
    pub active: bool,
    pub metadata_uri: String,
}

use soroban_sdk::contracttype;
use soroban_sdk::Address;

/// Storage keys for the Escrow contract.
#[contracttype]
pub enum DataKey {
    /// The token contract address (instance storage).
    TokenAddress,
    /// The registry contract address (instance storage).
    RegistryAddress,
    /// The receipt contract address, wired post-deploy (instance storage).
    ReceiptAddress,
    /// Per-caller, per-provider balance: (caller, provider) -> i128
    Balance(Address, Address),
}

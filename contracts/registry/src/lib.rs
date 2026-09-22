//! Registry contract — provider catalogue for the Meterly metering layer.
//!
//! # Purpose
//! Maintains the on-chain registry of API providers: their price-per-call (in USDC
//! stroops, i.e. 7-decimal integer units), active/inactive status, and a URI pointing
//! to off-chain metadata (display name, endpoint base URL, documentation link).
//!
//! # Storage & TTL policy
//! - `DataKey::Admin` — instance storage, no expiry concern (instance TTL = contract TTL).
//! - `DataKey::Provider(addr)` — **persistent storage**. TTL is extended on every write
//!   and on every read via `get_provider`. If a provider entry's TTL is allowed to expire
//!   (i.e. no write/read touches it for the ledger-defined persistent TTL window), the
//!   entry becomes inaccessible. Callers should be aware that a provider entry *can* expire
//!   if it is never touched; the off-chain indexer should monitor and refresh TTLs for
//!   active providers by calling `get_provider` periodically.

#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env};

use errors::Error;
use types::DataKey;

// Persistent TTL: extend to ~1 year of ledgers (5 second ledgers → 6_311_520 ledgers/year).
const PERSISTENT_BUMP_AMOUNT: u32 = 6_312_000;
const PERSISTENT_THRESHOLD: u32 = 6_311_000;

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Initialises the registry with an admin address.
    ///
    /// Must be called exactly once. Requires authorisation from `admin`.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if the admin is already set.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }
}

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
mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, String};

use errors::Error;
use events::{ActiveChanged, PriceUpdated, ProviderRegistered};
use types::{DataKey, ProviderInfo};

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

    /// Registers a new API provider or updates an existing registration.
    ///
    /// Requires authorisation from `provider`. The `price_per_call` must be strictly
    /// positive (> 0). Sets the provider as `active: true` on registration. Extends
    /// persistent storage TTL.
    ///
    /// # Errors
    /// - [`Error::InvalidPrice`] if `price_per_call <= 0`.
    pub fn register_provider(
        env: Env,
        provider: Address,
        price_per_call: i128,
        metadata_uri: String,
    ) -> Result<(), Error> {
        provider.require_auth();

        if price_per_call <= 0 {
            return Err(Error::InvalidPrice);
        }

        let info = ProviderInfo {
            owner: provider.clone(),
            price_per_call,
            active: true,
            metadata_uri,
        };

        let key = DataKey::Provider(provider.clone());
        env.storage()
            .persistent()
            .set(&key, &info);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        ProviderRegistered {
            provider,
            price_per_call,
        }
        .publish(&env);

        Ok(())
    }

    /// Updates the price-per-call for an existing provider.
    ///
    /// Requires authorisation from `provider`. The `new_price` must be strictly
    /// positive (> 0). Extends persistent storage TTL.
    ///
    /// # Errors
    /// - [`Error::ProviderNotFound`] if the provider is not registered.
    /// - [`Error::InvalidPrice`] if `new_price <= 0`.
    pub fn update_price(
        env: Env,
        provider: Address,
        new_price: i128,
    ) -> Result<(), Error> {
        provider.require_auth();

        if new_price <= 0 {
            return Err(Error::InvalidPrice);
        }

        let key = DataKey::Provider(provider.clone());
        let mut info: ProviderInfo = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProviderNotFound)?;

        let old_price = info.price_per_call;
        info.price_per_call = new_price;

        env.storage()
            .persistent()
            .set(&key, &info);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        PriceUpdated {
            provider,
            old_price,
            new_price,
        }
        .publish(&env);

        Ok(())
    }

    /// Toggles a provider's active status.
    ///
    /// Requires authorisation from `provider`. Extends persistent storage TTL.
    ///
    /// # Errors
    /// - [`Error::ProviderNotFound`] if the provider is not registered.
    pub fn set_active(env: Env, provider: Address, active: bool) -> Result<(), Error> {
        provider.require_auth();

        let key = DataKey::Provider(provider.clone());
        let mut info: ProviderInfo = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProviderNotFound)?;

        info.active = active;

        env.storage()
            .persistent()
            .set(&key, &info);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        ActiveChanged {
            provider,
            active,
        }
        .publish(&env);

        Ok(())
    }

    /// Reads a provider's information without modifying any state.
    ///
    /// This is a read-only operation that does not require authorization.
    /// The persistent TTL is extended, ensuring the entry does not expire from
    /// normal querying.
    ///
    /// # Errors
    /// - [`Error::ProviderNotFound`] if the provider is not registered.
    pub fn get_provider(env: Env, provider: Address) -> Result<ProviderInfo, Error> {
        let key = DataKey::Provider(provider);
        let info: ProviderInfo = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProviderNotFound)?;

        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        Ok(info)
    }
}

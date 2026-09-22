//! Receipt contract — immutable record of API calls with provider co-signature.
//!
//! # Purpose
//! Records API call receipts anchored to settlements in escrow. Each receipt requires
//! authorization from both caller and provider (dual-sign), ensuring providers cannot
//! unilaterally claim calls were made. Nonce-based replay protection per caller.
//!
//! # Storage & TTL policy
//! - `DataKey::EscrowAddress`, `DataKey::RegistryAddress` — instance storage, no expiry.
//! - `DataKey::Nonce(caller)` — persistent storage. TTL extended on every receipt submission.
//! - `DataKey::Receipt(receipt_hash)` — persistent storage. TTL extended on reads.
//!   Receipts can expire if not queried, but submitted receipts have their TTL bumped.

#![no_std]

mod errors;
mod events;
mod provider_info;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

use errors::Error;
use events::ReceiptSubmitted;
use types::{DataKey, ReceiptRecord};

// Persistent TTL: extend to ~1 year of ledgers (5 second ledgers → 6_311_520 ledgers/year).
const PERSISTENT_BUMP_AMOUNT: u32 = 6_312_000;
const PERSISTENT_THRESHOLD: u32 = 6_311_000;

#[contract]
pub struct ReceiptContract;

#[contractimpl]
impl ReceiptContract {
    /// Initialises the receipt contract with escrow and registry addresses.
    ///
    /// Must be called exactly once. No authorization required beyond transaction auth.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if already initialized.
    pub fn initialize(env: Env, escrow: Address, registry: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::EscrowAddress) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::EscrowAddress, &escrow);
        env.storage().instance().set(&DataKey::RegistryAddress, &registry);

        Ok(())
    }

    /// Submits a receipt, settled immediately with dual authorization.
    ///
    /// Both `caller` and `provider` must authorize this transaction. Validates:
    /// - Nonce is strictly greater than caller's last nonce (replay protection).
    /// - Amount does not exceed provider's price-per-call from the registry.
    /// - Receipt with this hash does not already exist.
    ///
    /// On success, atomically calls `escrow.settle(caller, provider, amount)` to transfer
    /// funds. If `settle` fails, the entire transaction reverts and no receipt is stored.
    /// If the transaction succeeds but the provider's entry expires from the registry,
    /// the receipt is still valid but may indicate a stale provider record.
    ///
    /// # Errors
    /// - [`Error::InvalidNonce`] if `nonce <= Nonce(caller)`.
    /// - [`Error::PriceMismatch`] if `amount > provider.price_per_call`.
    /// - [`Error::ReceiptExists`] if a receipt with `receipt_hash` already exists.
    pub fn submit_receipt(
        env: Env,
        caller: Address,
        provider: Address,
        amount: i128,
        nonce: u64,
        receipt_hash: BytesN<32>,
    ) -> Result<(), Error> {
        // Require authorization from both caller and provider
        caller.require_auth();
        provider.require_auth();

        // Check nonce: must be strictly increasing per caller
        let nonce_key = DataKey::Nonce(caller.clone());
        let last_nonce: u64 = env
            .storage()
            .persistent()
            .get(&nonce_key)
            .unwrap_or(0);

        if nonce <= last_nonce {
            return Err(Error::InvalidNonce);
        }

        // Verify provider exists and check price
        let registry_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::RegistryAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let registry_client = registry_client::RegistryContractClient::new(&env, &registry_addr);
        let provider_info: provider_info::ProviderInfo = registry_client.get_provider(&provider);

        if amount > provider_info.price_per_call {
            return Err(Error::PriceMismatch);
        }

        // Check receipt doesn't already exist
        let receipt_key = DataKey::Receipt(receipt_hash.clone());
        if env.storage().persistent().has(&receipt_key) {
            return Err(Error::ReceiptExists);
        }

        // Call escrow.settle to transfer funds atomically
        let escrow_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let escrow_client = escrow_client::EscrowContractClient::new(&env, &escrow_addr);
        escrow_client.settle(&caller, &provider, &amount);

        // Store receipt record
        let timestamp = env.ledger().timestamp();
        let record = ReceiptRecord {
            caller: caller.clone(),
            provider: provider.clone(),
            amount,
            nonce,
            timestamp,
        };

        env.storage().persistent().set(&receipt_key, &record);
        env.storage()
            .persistent()
            .extend_ttl(&receipt_key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        // Update caller's nonce
        env.storage().persistent().set(&nonce_key, &nonce);
        env.storage()
            .persistent()
            .extend_ttl(&nonce_key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        ReceiptSubmitted {
            caller,
            provider,
            receipt_hash,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Reads a receipt record by its hash.
    ///
    /// This is read-only. Returns the stored ReceiptRecord or an error if not found.
    /// Extends the persistent TTL to keep the entry alive.
    ///
    /// # Errors
    /// - [`Error::ReceiptNotFound`] if the receipt_hash is not stored.
    pub fn get_receipt(
        env: Env,
        receipt_hash: BytesN<32>,
    ) -> Result<ReceiptRecord, Error> {
        let key = DataKey::Receipt(receipt_hash);
        let record: ReceiptRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ReceiptNotFound)?;

        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        Ok(record)
    }

    /// Reads the current nonce for a caller.
    ///
    /// This is read-only. Returns 0 if the caller has never submitted a receipt.
    /// Extends the persistent TTL to keep the entry alive.
    pub fn get_nonce(env: Env, caller: Address) -> u64 {
        let key = DataKey::Nonce(caller);
        let nonce: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        if nonce > 0 {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        }

        nonce
    }
}

// ─ Cross-contract client modules ──────────────────────────────────────────

mod registry_client {
    use soroban_sdk::{contractclient, Address, Env};
    use crate::provider_info::ProviderInfo;

    #[contractclient(name = "RegistryContractClient")]
    pub trait RegistryContract {
        fn get_provider(env: &Env, provider: Address) -> ProviderInfo;
    }
}

mod escrow_client {
    use soroban_sdk::{contractclient, Address, Env};

    #[contractclient(name = "EscrowContractClient")]
    pub trait EscrowContract {
        fn settle(env: &Env, caller: Address, provider: Address, amount: i128);
    }
}

#[cfg(test)]
mod test {}

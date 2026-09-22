//! Escrow contract — balance sheet for API call settlement on Meterly.
//!
//! # Purpose
//! Holds caller deposits mapped per (caller, provider) pair. Manages deposits, withdrawals,
//! and settlements. Only the receipt contract (wired at `set_receipt_contract`) may call
//! `settle`, which transfers funds from caller to provider atomically with receipt recording.
//!
//! # Storage & TTL policy
//! - `DataKey::TokenAddress`, `DataKey::RegistryAddress`, `DataKey::ReceiptAddress` —
//!   instance storage, no expiry concern.
//! - `DataKey::Balance(caller, provider)` — persistent storage. TTL is extended on every
//!   write and on every read. A balance entry can expire if untouched, making it temporarily
//!   inaccessible; callers should be aware of this possibility, though normal usage patterns
//!   (deposits → settlements → withdrawals within a session) will prevent expiry.

#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};

use errors::Error;
use events::{Deposited, ReceiptContractSet, Settled, Withdrawn};
use types::DataKey;

// Persistent TTL: extend to ~1 year of ledgers (5 second ledgers → 6_311_520 ledgers/year).
const PERSISTENT_BUMP_AMOUNT: u32 = 6_312_000;
const PERSISTENT_THRESHOLD: u32 = 6_311_000;

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Initialises the escrow with token and registry contract addresses.
    ///
    /// Must be called exactly once before any deposits are accepted. Requires authorisation
    /// from the deployer (typically verified via transaction signature). No `admin` address
    /// is stored; post-deploy wiring of the receipt contract is handled separately via
    /// `set_receipt_contract`.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if already initialized.
    pub fn initialize(env: Env, token: Address, registry: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::TokenAddress) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::RegistryAddress, &registry);

        Ok(())
    }

    /// Wires the receipt contract address post-deploy.
    ///
    /// This must be called exactly once after both `escrow` and `receipt` are deployed,
    /// and before any receipts can be submitted. Requires authorisation from `admin`.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if receipt contract already wired.
    pub fn set_receipt_contract(env: Env, admin: Address, receipt: Address) -> Result<(), Error> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::ReceiptAddress) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::ReceiptAddress, &receipt);

        ReceiptContractSet {
            receipt: receipt.clone(),
        }
        .publish(&env);

        Ok(())
    }

    /// Deposits funds from a caller into escrow for a specific provider.
    ///
    /// Requires authorisation from `caller`. Verifies that the provider is registered
    /// in the registry contract. Transfers `amount` of the token from caller to this
    /// contract. Increments `Balance(caller, provider)` and extends TTL.
    ///
    /// # Errors
    /// - [`Error::InvalidAmount`] if `amount <= 0`.
    /// - [`Error::ProviderNotFound`] if the provider is not registered in the registry.
    pub fn deposit(
        env: Env,
        caller: Address,
        provider: Address,
        amount: i128,
    ) -> Result<(), Error> {
        caller.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Verify provider exists in registry
        let registry_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::RegistryAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let registry_client =
            crate::registry_client::RegistryContractClient::new(&env, &registry_addr);
        registry_client.get_provider(&provider);

        // Transfer token from caller to this contract
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let token_client = TokenClient::new(&env, &token_addr);
        token_client.transfer(
            &caller,
            &env.current_contract_address(),
            &amount,
        );

        // Update balance
        let key = DataKey::Balance(caller.clone(), provider.clone());
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        let new_balance = current_balance
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        env.storage().persistent().set(&key, &new_balance);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        Deposited {
            caller,
            provider,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Withdraws unused funds from escrow back to the caller.
    ///
    /// Requires authorisation from `caller`. Decrements `Balance(caller, provider)`.
    /// Transfers `amount` back to `caller`.
    ///
    /// # Errors
    /// - [`Error::InsufficientBalance`] if caller's balance is less than `amount`.
    pub fn withdraw(
        env: Env,
        caller: Address,
        provider: Address,
        amount: i128,
    ) -> Result<(), Error> {
        caller.require_auth();

        let key = DataKey::Balance(caller.clone(), provider.clone());
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        if current_balance < amount {
            return Err(Error::InsufficientBalance);
        }

        let new_balance = current_balance - amount;
        if new_balance > 0 {
            env.storage().persistent().set(&key, &new_balance);
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        } else {
            // Remove entry if balance becomes zero
            env.storage().persistent().remove(&key);
        }

        // Transfer token from this contract to caller
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let token_client = TokenClient::new(&env, &token_addr);
        token_client.transfer(
            &env.current_contract_address(),
            &caller,
            &amount,
        );

        Withdrawn {
            caller,
            provider,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Settles funds from caller's balance to the provider.
    ///
    /// Only the receipt contract (verified via caller checking) may call this function.
    /// The receipt contract must pass its own address as the caller for authentication.
    /// Decrements `Balance(caller, provider)` and transfers `amount` to `provider`.
    /// Called atomically with receipt recording in `receipt::submit_receipt`; if this
    /// function fails, the receipt is not stored.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] if the calling contract is not the authorized receipt contract.
    /// - [`Error::InsufficientBalance`] if caller's balance is less than `amount`.
    pub fn settle(
        env: Env,
        caller: Address,
        provider: Address,
        amount: i128,
    ) -> Result<(), Error> {
        // Verify receipt contract authorization by checking that a contract matching receipt_addr is invoking
        let receipt_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::ReceiptAddress)
            .ok_or(Error::AlreadyInitialized)?;

        // The receipt contract must require_auth on itself when calling settle
        receipt_addr.require_auth();

        let key = DataKey::Balance(caller.clone(), provider.clone());
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        if current_balance < amount {
            return Err(Error::InsufficientBalance);
        }

        let new_balance = current_balance - amount;
        if new_balance > 0 {
            env.storage().persistent().set(&key, &new_balance);
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        } else {
            // Remove entry if balance becomes zero
            env.storage().persistent().remove(&key);
        }

        // Transfer token from this contract to provider
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::AlreadyInitialized)?;

        let token_client = TokenClient::new(&env, &token_addr);
        token_client.transfer(
            &env.current_contract_address(),
            &provider,
            &amount,
        );

        Settled {
            caller,
            provider,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Reads the balance for a caller-provider pair without modifying state.
    ///
    /// This is a read-only operation. If no balance exists, returns 0.
    /// Extends the persistent TTL to keep the entry alive.
    pub fn get_balance(
        env: Env,
        caller: Address,
        provider: Address,
    ) -> i128 {
        let key = DataKey::Balance(caller, provider);
        let balance: i128 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        if balance > 0 {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        }

        balance
    }
}

// ─ Cross-contract client module ──────────────────────────────────────────

mod registry_client {
    use soroban_sdk::{contractclient, Address, Env};

    #[contractclient(name = "RegistryContractClient")]
    pub trait RegistryContract {
        fn get_provider(env: &Env, provider: Address);
    }
}

#[cfg(test)]
mod test {}

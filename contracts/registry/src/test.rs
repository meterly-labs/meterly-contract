#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::{errors::Error, events::{ActiveChanged, PriceUpdated, ProviderRegistered}, types::ProviderInfo, RegistryContract, RegistryContractClient};

fn setup() -> (Env, RegistryContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);
    (env, client)
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn test_initialize_ok() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
}

#[test]
fn test_initialize_already_initialized() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let err = client.try_initialize(&admin).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

// ── register_provider ────────────────────────────────────────────────────────

#[test]
fn test_register_provider_ok() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let price = 1_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &price, &uri);

    let info = client.get_provider(&provider);
    assert_eq!(info.owner, provider);
    assert_eq!(info.price_per_call, price);
    assert_eq!(info.active, true);
    assert_eq!(info.metadata_uri, uri);
}

#[test]
fn test_register_provider_invalid_price_zero() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let uri = String::from_slice(&env, "https://api.example.com");

    let err = client.try_register_provider(&provider, &0, &uri).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidPrice);
}

#[test]
fn test_register_provider_invalid_price_negative() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let uri = String::from_slice(&env, "https://api.example.com");

    let err = client.try_register_provider(&provider, &(-1000i128), &uri).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidPrice);
}

#[test]
fn test_register_provider_emits_event() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let price = 500_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &price, &uri);

    let events = env.events().all();
    let last = events.last().unwrap();
    let (event,) = last.1.parse::<(ProviderRegistered,)>().unwrap();
    assert_eq!(event.provider, provider);
    assert_eq!(event.price_per_call, price);
}

// ── update_price ─────────────────────────────────────────────────────────────

#[test]
fn test_update_price_ok() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let initial_price = 1_000_000i128;
    let new_price = 2_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &initial_price, &uri);
    client.update_price(&provider, &new_price);

    let info = client.get_provider(&provider);
    assert_eq!(info.price_per_call, new_price);
}

#[test]
fn test_update_price_provider_not_found() {
    let (env, client) = setup();
    let provider = Address::generate(&env);

    let err = client.try_update_price(&provider, &1_000_000i128).unwrap_err().unwrap();
    assert_eq!(err, Error::ProviderNotFound);
}

#[test]
fn test_update_price_invalid_price() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let initial_price = 1_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &initial_price, &uri);

    let err = client.try_update_price(&provider, &0).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidPrice);
}

#[test]
fn test_update_price_emits_event() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let initial_price = 1_000_000i128;
    let new_price = 2_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &initial_price, &uri);
    env.events().all().pop(); // Remove register event

    client.update_price(&provider, &new_price);

    let events = env.events().all();
    let last = events.last().unwrap();
    let (event,) = last.1.parse::<(PriceUpdated,)>().unwrap();
    assert_eq!(event.provider, provider);
    assert_eq!(event.old_price, initial_price);
    assert_eq!(event.new_price, new_price);
}

// ── set_active ───────────────────────────────────────────────────────────────

#[test]
fn test_set_active_true() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let price = 1_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &price, &uri);
    client.set_active(&provider, &false);
    client.set_active(&provider, &true);

    let info = client.get_provider(&provider);
    assert_eq!(info.active, true);
}

#[test]
fn test_set_active_false() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let price = 1_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &price, &uri);
    client.set_active(&provider, &false);

    let info = client.get_provider(&provider);
    assert_eq!(info.active, false);
}

#[test]
fn test_set_active_provider_not_found() {
    let (env, client) = setup();
    let provider = Address::generate(&env);

    let err = client.try_set_active(&provider, &false).unwrap_err().unwrap();
    assert_eq!(err, Error::ProviderNotFound);
}

#[test]
fn test_set_active_emits_event() {
    let (env, client) = setup();
    let provider = Address::generate(&env);
    let price = 1_000_000i128;
    let uri = String::from_slice(&env, "https://api.example.com");

    client.register_provider(&provider, &price, &uri);
    env.events().all().pop(); // Remove register event

    client.set_active(&provider, &false);

    let events = env.events().all();
    let last = events.last().unwrap();
    let (event,) = last.1.parse::<(ActiveChanged,)>().unwrap();
    assert_eq!(event.provider, provider);
    assert_eq!(event.active, false);
}


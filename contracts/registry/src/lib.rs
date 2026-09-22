//! Registry contract — provider catalogue for the Meterly metering layer.
//!
//! # Purpose
//! Maintains the on-chain registry of API providers: their price-per-call,
//! active/inactive status, and a URI pointing to off-chain metadata.

#![no_std]

mod types;

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {}

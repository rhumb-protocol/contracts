//! The reference tests of spec A-001 §4 that concern this adapter: test 2 (`post`, the two
//! roles, `observe`, and the instance TTL) and this adapter's half of test 3 (the adapter
//! neither narrows nor widens what the pool accepts).
//!
//! The adapter is driven in two modes, as the engine's tests are: `Wasm` deploys the
//! adapter's own Wasm with its constructor and calls it through a generated client, which is
//! the only mode that proves the interface, the XDR round trip and the authorisation the host
//! enforces; `Native` calls the same entry points inside `Env::as_contract` on the same
//! instance, which is what gives the crate the line coverage A-001 §6 asks for. The [`both!`]
//! macro instantiates a test in both modes.
//!
//! Tests of a refused authorisation run in `Wasm` mode only: natively a failing
//! `require_auth` is a panic rather than an error the host returns, so the assertion would be
//! about the test environment rather than about the contract. The `require_auth` calls
//! themselves are covered natively by every test that does authorise.
//!
//! The Wasm is read from the workspace target directory (`cargo build --workspace --release
//! --target wasm32v1-none` produces it). When it is missing the tests print a notice and
//! pass, unless `ADAPTER_WASM_REQUIRED` is set, in which case they fail; CI sets it after the
//! Wasm build, so a green CI cannot mean that the tests found nothing to run.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::error::Error;
use crate::types::Posting;
use soroban_sdk::{Address, Env};

/// The contract's interface as the tests see it: one entry per entry point of spec §3.3 after
/// the constructor. The generated client drives the Wasm; the constructor is driven by the
/// deployment itself.
#[soroban_sdk::contractclient(name = "IssuerClient")]
#[allow(dead_code)]
pub trait IssuerInterface {
    fn post(env: Env, price: i128, timestamp: u64) -> Result<(), Error>;
    fn observe(env: Env) -> Option<(i128, u32, u64)>;
    fn set_keeper(env: Env, keeper: Address) -> Result<(), Error>;
    fn set_governance(env: Env, governance: Address) -> Result<(), Error>;
    fn keeper(env: Env) -> Option<Address>;
    fn governance(env: Env) -> Option<Address>;
    fn decimals(env: Env) -> Option<u32>;
    fn latest(env: Env) -> Option<Posting>;
    fn version(env: Env) -> u32;
}

/// How a test drives the adapter: through its Wasm, or natively inside `as_contract`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Wasm,
    Native,
}

/// Instantiates a test body `|f: &Fixture| { .. }` once per mode.
macro_rules! both {
    ($name:ident, $body:expr) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;

            #[test]
            fn wasm() {
                if let Some(f) = Fixture::new(Mode::Wasm) {
                    let body: fn(&Fixture) = $body;
                    body(&f);
                }
            }

            #[test]
            fn native() {
                if let Some(f) = Fixture::new(Mode::Native) {
                    let body: fn(&Fixture) = $body;
                    body(&f);
                }
            }
        }
    };
}

pub mod harness;
mod t01_post;
mod t02_roles;
mod t03_pool_rule;
mod t04_ttl;
mod t05_write_set;
mod t06_constructor_once;
mod t07_event_schema;
mod t08_constructor_auth;
mod t09_constructor_event;
mod t10_init_event;

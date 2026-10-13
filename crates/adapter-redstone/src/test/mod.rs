//! The reference tests of spec A-001 §4 that concern this adapter: test 1 (`observe` against a
//! mock SEP-40 feed, and the constructor's refusals) and this adapter's half of test 3 (the
//! adapter neither narrows nor widens what the pool accepts).
//!
//! Every test runs in the Soroban test environment with the mock feed of `crates/sep40-mock`
//! as the oracle. The adapter is driven in two modes, as the engine's tests are: `Wasm`
//! deploys the adapter's own Wasm with its constructor and calls it through a generated
//! client, which is the only mode that proves the interface and the XDR round trip;
//! `Native` calls the same entry points inside `Env::as_contract` on a shell instance, which
//! is what gives the crate the line coverage A-001 §6 asks for. The [`both!`] macro
//! instantiates a test in both modes.
//!
//! The Wasm files are read from the workspace target directory (`cargo build --workspace
//! --release --target wasm32v1-none` produces them). When a file is missing the tests print a
//! notice and pass, unless `ADAPTER_WASM_REQUIRED` is set, in which case they fail; CI sets it
//! after the Wasm build so that a green CI never means "the tests found nothing to run".

#![allow(
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::sep40::Asset;
use soroban_sdk::{Address, Env};

/// The contract's interface as the tests see it: one entry per entry point of spec §3.2 after
/// the constructor. The generated client drives the Wasm; the constructor is driven by the
/// deployment itself.
#[soroban_sdk::contractclient(name = "AdapterClient")]
#[allow(dead_code)]
pub trait AdapterInterface {
    fn observe(env: Env) -> Option<(i128, u32, u64)>;
    fn oracle(env: Env) -> Option<Address>;
    fn asset(env: Env) -> Option<Asset>;
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
mod t01_observe;
mod t02_constructor;
mod t03_pool_rule;
mod t04_constructor_once;
mod t05_constructor_event;
mod t06_init_event;

pub use harness::Fixture;

//! A test-only SEP-40 price feed, the mock oracle the reference tests of spec A-001 §4 point
//! the adapters at.
//!
//! It is not part of Rhumb's chain code and is never deployed on a public network: it exists
//! so that `adapter-redstone` can be driven against a feed whose answers a test chooses,
//! including the answers a real feed may give and a test must cover (`None`, a call that
//! fails, decimals above the pool's bound).
//!
//! # Why a crate of its own
//!
//! The mock has to be a registered contract, and on the host `contractimpl` with the SDK's
//! `testutils` feature (which `cargo test --workspace` enables through feature unification)
//! generates a registration constructor carrying `#[allow(unsafe_code)]`, which the
//! workspace's `forbid(unsafe_code)` rejects. The `contract` and `contractimpl` attributes
//! are therefore applied only when compiling for Wasm, and the tests register this crate's
//! Wasm from the workspace target directory. The same reasoning is why the engine keeps its
//! mocks in a crate of their own; no code is taken from there.
//!
//! # The interface
//!
//! The part of SEP-40 an adapter uses: `decimals() -> u32` and `lastprice(asset) ->
//! Option<PriceData>`. A test sets the answers with `set_decimals`, `set_price` and
//! `clear_price`, and makes either call fail with `set_failing`.
//!
//! # Why the readings are per asset
//!
//! On a real feed the asset argument selects the feed: ask for the wrong one and you get
//! another token's price, at a plausible scale and a valid timestamp, which a pool would
//! accept without a murmur. A mock that answered the same reading for every asset would let an
//! adapter pass its whole suite while asking for a constant, a wrong variant or a different
//! address. So `set_price_for` stores a reading against one asset and `lastprice` answers only
//! for that asset. `set_price` remains for the tests where the asset is not the point; a
//! per-asset reading always wins over it.
//!
//! # Provenance
//!
//! Nothing here is ported. The interface follows the published SEP-40 standard (Stellar
//! Ecosystem Proposal 40, `lastprice(asset) -> Option<PriceData { price: i128, timestamp:
//! u64 }>`, `decimals() -> u32`); the types are written from it.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::arithmetic_side_effects)]
#![allow(missing_docs)]

use soroban_sdk::{contracttype, Address, Env, Symbol};

/// SEP-40's asset: a token contract on Stellar, or a feed named by a symbol.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

/// SEP-40's reading: a price at the feed's scale, with the feed's own timestamp.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

/// The storage keys of the mock.
#[contracttype]
#[derive(Clone)]
pub enum MockKey {
    Decimals,
    /// The answer to `lastprice` for every asset without one of its own; absent means `None`.
    Price,
    /// The answer to `lastprice` for one asset, which wins over [`MockKey::Price`].
    PriceFor(Asset),
    /// When set, both `decimals` and `lastprice` fail.
    Failing,
}

/// A SEP-40 feed whose answers the test chooses.
#[cfg_attr(target_family = "wasm", soroban_sdk::contract)]
pub struct Sep40Mock;

#[cfg_attr(target_family = "wasm", soroban_sdk::contractimpl)]
impl Sep40Mock {
    /// Sets the scale `decimals` returns. Test-only: no authorisation.
    pub fn set_decimals(env: Env, decimals: u32) {
        env.storage().instance().set(&MockKey::Decimals, &decimals);
    }

    /// Sets the reading `lastprice` returns for every asset.
    pub fn set_price(env: Env, price: i128, timestamp: u64) {
        env.storage()
            .instance()
            .set(&MockKey::Price, &PriceData { price, timestamp });
    }

    /// Sets the reading `lastprice` returns for one asset only, as a real feed does.
    pub fn set_price_for(env: Env, asset: Asset, price: i128, timestamp: u64) {
        env.storage()
            .instance()
            .set(&MockKey::PriceFor(asset), &PriceData { price, timestamp });
    }

    /// Removes the reading, so that `lastprice` answers `None` as a feed without data does.
    pub fn clear_price(env: Env) {
        env.storage().instance().remove(&MockKey::Price);
    }

    /// Makes both calls fail from now on, as an oracle that is down does.
    pub fn set_failing(env: Env, failing: bool) {
        env.storage().instance().set(&MockKey::Failing, &failing);
    }

    /// SEP-40 `decimals`.
    pub fn decimals(env: Env) -> u32 {
        Self::refuse_when_failing(&env);
        env.storage()
            .instance()
            .get(&MockKey::Decimals)
            .unwrap_or(0)
    }

    /// SEP-40 `lastprice`: the reading stored for this asset, else the one stored for every
    /// asset, else `None`.
    pub fn lastprice(env: Env, asset: Asset) -> Option<PriceData> {
        Self::refuse_when_failing(&env);
        env.storage()
            .instance()
            .get(&MockKey::PriceFor(asset))
            .or_else(|| env.storage().instance().get(&MockKey::Price))
    }

    fn refuse_when_failing(env: &Env) {
        if env
            .storage()
            .instance()
            .get(&MockKey::Failing)
            .unwrap_or(false)
        {
            soroban_sdk::panic_with_error!(env, soroban_sdk::Error::from_contract_error(1));
        }
    }
}

//! The part of SEP-40 this adapter calls (spec A-001 §3.2).
//!
//! SEP-40 is the Stellar oracle interface: `decimals() -> u32` fixes the scale of every price
//! the feed publishes, and `lastprice(asset) -> Option<PriceData>` is the newest reading for
//! one asset, or `None` when the feed holds none. The adapter uses those two and nothing
//! else: not `price(asset, timestamp)`, not `prices(asset, records)`, not `assets()`, because
//! a pool's `refresh` wants one reading and the fewer calls the adapter makes the less of the
//! pool's call budget it spends.
//!
//! [`Asset`] and [`PriceData`] are written from the standard and must keep its XDR shape: they
//! cross the contract boundary to a feed this repository does not own, so a field order or a
//! variant order changed here would silently misread a live oracle.
//!
//! # Provenance
//!
//! Nothing here is ported; the types and the client are written from the published SEP-40
//! standard.

// The SDK's `contracttype`, `contracterror`, `contractclient` and `contractimpl` macros
// generate items without doc comments, which the workspace's `missing_docs = "deny"` would
// reject; the items this file writes by hand are documented above each of them.
#![allow(missing_docs)]

use soroban_sdk::{contracttype, Address, Env, Symbol};

/// SEP-40's asset: `Stellar(Address)` names a token by its contract id, `Other(Symbol)` names
/// a feed by its ticker. Every pool of Rhumb uses `Stellar(<token contract id>)`, the same id
/// the brand's record of the token holds, which its adapter and pool deployments both read, so
/// that the feed an adapter reads is tied to the token the pool holds and not to a name that
/// could be pointed elsewhere.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

/// SEP-40's reading: `price` at the scale `decimals()` returns, `timestamp` the feed's own
/// publication time. The standard defines the timestamp in seconds; this adapter does not
/// check that and does not convert (spec A-001 §3.2), because a conversion would be a
/// decision about the source.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

/// The feed as this adapter calls it. The generated `Sep40Client` propagates a failure of the
/// feed as a failure of the adapter's own call, which is what spec §3.2 asks for: a pool
/// treats a failing adapter as a source without an observation (E-007 §3.2).
#[soroban_sdk::contractclient(name = "Sep40Client")]
#[allow(dead_code)]
pub trait Sep40 {
    fn decimals(env: Env) -> u32;
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;
}

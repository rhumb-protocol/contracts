//! The rate adapter that wraps a SEP-40 price feed, built to spec `spec/A-001-rate-adapters.md`
//! §3.2.
//!
//! One deployed instance is one source of one band pool: the pool calls [`observe`] on it in
//! `refresh` (E-007 §3.2) and hands what comes back to `Observation::from_sep40` (E-003 §3.2).
//!
//! # What the contract decides
//!
//! Nothing. It converts nothing, bounds nothing and stores nothing after its constructor. It
//! reads the feed's `decimals()` and its `lastprice(asset)` and returns the two numbers and
//! the feed's own timestamp exactly as received; a `None` from the feed is a `None` from the
//! adapter, and a feed call that fails fails `observe`, which the pool treats as a source
//! without an observation. Every judgement about a price -- the sign, the scale, the range of
//! the rate, the age of the reading, the step against the stored band -- belongs to the
//! engine, and a bound here would hide from the pool what the feed published.
//!
//! There is deliberately no arithmetic in this crate: not a scaling, not a unit conversion,
//! not a comparison against a previous value. The timestamp is passed through in whatever
//! unit the feed publishes.
//!
//! # Immutability
//!
//! The oracle and the asset are fixed by the constructor and there is no setter and no
//! upgrade: a source is replaced by deploying a new adapter and calling the pool's `set_rate`
//! (E-007 §3.2), never by editing a live one, so that what a pool reads can never change
//! under it without governance having signed for the change.
//!
//! # No TTL code
//!
//! The adapter's instance entry is kept alive from outside by a weekly extend-footprint-TTL
//! operation (A-001 §3.4), a ledger operation that needs no contract call, so the contract
//! carries no TTL handling of its own.
//!
//! # Host and Wasm
//!
//! The `contract` and `contractimpl` attributes are applied only when compiling for Wasm: on
//! the host, `contractimpl` with the SDK's `testutils` feature (which `cargo test
//! --workspace` enables through feature unification) generates a registration constructor
//! carrying `#[allow(unsafe_code)]`, which the workspace's `forbid(unsafe_code)` rejects. The
//! tests drive the contract's Wasm through a generated client and run the same entry points
//! natively inside `Env::as_contract` for the line coverage A-001 §6 asks for.
//!
//! # Provenance
//!
//! Nothing here is ported. The SEP-40 types follow the published standard (Stellar Ecosystem
//! Proposal 40); the engine's interface is read in the engine's repository (E-007 §5) and no
//! engine code is copied.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::arithmetic_side_effects)]

#[cfg(test)]
extern crate std;

pub mod contract;
pub mod error;
pub mod events;
pub mod sep40;
pub mod types;

pub use contract::RedstoneAdapter;

/// The spec revision this Wasm implements: a constant of the Wasm, not a storage entry. It is
/// raised by every change to the Wasm's interface -- an entry point, an argument, a return
/// type, an error code, an event's topics or its data fields -- because whatever reads this
/// adapter depends on all of them. `1` is the first implementation of A-001 §3.2; `2` adds the
/// constructor's `init` event (A-001 §3.2 as amended 2026-09-29).
pub const VERSION: u32 = 2;

/// The most decimals an adapter may hand a pool: the pool refuses more (E-003 §3.2), so an
/// adapter on a feed with a coarser scale would be a source the pool could never accept, and
/// the constructor refuses to create one.
pub const DECIMALS_MAX: u32 = 18;

#[cfg(test)]
mod test;

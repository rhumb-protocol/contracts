//! The rate adapter that holds an issuer's published price, built to spec
//! `spec/A-001-rate-adapters.md` §3.3.
//!
//! One deployed instance is one issuer source: the keeper reads the issuer off chain and posts
//! what it read here, and the pools of that asset call [`observe`] on it in `refresh` (E-007
//! §3.2). One instance serves every pool of its asset, so that all of them read one number.
//!
//! # What the contract decides
//!
//! Almost nothing, and deliberately so. `post` refuses three things and three only: a price
//! that is not positive, a timestamp older than the stored one, and a timestamp later than
//! the ledger's. Those three are not judgements about the price; they are what makes the
//! stored pair meaningful at all -- a price at or below zero is not a price, a reading from
//! before the one already held would take the adapter backwards, and a publication time in the
//! future is not a publication time. A timestamp equal to the stored one is accepted (§3.3,
//! amended 2026-09-16), so that a price corrected under an unchanged publication time can be
//! posted; whether such a post is worth its fee is the keeper's question (§3.4). Everything that *is* a judgement about the price -- how far it may step,
//! how far it may sit from the other source, how old it may be before the pool stops trusting
//! it -- belongs to the engine (E-003), and applying any of it here would hide from the pool
//! what the keeper actually posted, which is the one thing a second source exists to reveal.
//!
//! The contract cannot check the thing that matters most about a post, which is that the
//! timestamp is the issuer's own publication time and not the moment the keeper happened to
//! read, nor the NAV's valuation time. Nothing on chain can. The keeper's readers carry that
//! rule and the keeper's tests hold them to it (spec §3.4, §4 test 5).
//!
//! # Two keys, two roles
//!
//! The keeper is a hot key on a machine that runs unattended; governance is not. So the keeper
//! may do exactly one thing, `post`, and governance may replace the keeper without touching
//! anything else, which is what makes a stolen keeper key an incident that is survived by one
//! transaction. Governance can hand its own role on as well, for the move to a multisig
//! (E-007 §3.6).
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
//! Nothing here is ported. The adapter interface is read in `wadrock/core` (E-007 §5) and no
//! engine code is copied; the TTL values are the reference values E-007 §3.1 states, restated
//! here because this contract is not built from the engine's crates.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::arithmetic_side_effects)]

#[cfg(test)]
extern crate std;

pub mod contract;
pub mod error;
pub mod events;
pub mod types;

pub use contract::IssuerAdapter;

/// The spec revision this Wasm implements: a constant of the Wasm, not a storage entry. It is
/// raised by every change to the Wasm's interface -- an entry point, an argument, a return
/// type, an error code, an event's topics or its data fields, because whatever decodes this
/// adapter's events depends on all of them. `1` is the first implementation of A-001 §3.3;
/// `2` adds the constructor's `init` event (A-001 §3.3 as amended 2026-09-29).
pub const VERSION: u32 = 2;

/// The most decimals this adapter may be created with: the pool refuses more (E-003 §3.2), so
/// an adapter at a coarser scale could only ever be a source the pool skips. Every source of
/// Rhumb is deployed at 18 and the keeper scales every reading to 18 before posting (spec
/// §3.4), so that one number means the same thing at every source of every pool.
pub const DECIMALS_MAX: u32 = 18;

/// The instance TTL a `post` extends to when the entry is below [`INSTANCE_TTL_THRESHOLD`]:
/// 30 days at five-second ledgers, the reference value of E-007 §3.1, so that an adapter and
/// the pools reading it expire on the same schedule. Between posts the keeper's weekly
/// extend-footprint-TTL operation keeps it alive (spec §3.4); this is the belt to that
/// braces, because an adapter that expires takes its price with it.
pub const INSTANCE_TTL: u32 = 30 * 24 * 60 * 60 / 5;

/// Below this remaining TTL a `post` extends the instance: half of [`INSTANCE_TTL`] (E-007
/// §3.1).
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_TTL / 2;

#[cfg(test)]
mod test;

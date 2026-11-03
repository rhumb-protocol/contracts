//! The error of the adapter (spec A-001 §3.3, the numbering shared by the two adapters).
//!
//! The adapters number their errors from `300`, which spec §3.3 asks for so that they "never
//! collide with the pool's (E-007 §3.5) or the router's". The first half holds: the pool's
//! codes are `3 ..= 21` and `100 ..= 12x`. The second half does not, and the reason is worth
//! writing down rather than repeating. The router's own codes are `200 ..= 214`, but the
//! errors of its planner, which the router surfaces, are `300 ..= 304` (the error enums of the
//! router and its planner in the engine's repository), so the adapters' block overlaps the
//! router's planner block exactly.
//!
//! On chain this changes nothing. A Soroban contract error is scoped to the contract that
//! raised it, an adapter and the router never appear in one call tree -- `refresh` reads the
//! adapters and swallows a failure (E-007 §3.2), and no router call reaches an adapter -- and
//! a client reads the code together with the contract it came from. So the numbers are kept
//! and the premise is corrected here rather than the codes renumbered: renumbering would
//! change this Wasm's interface for a collision that cannot occur, and the premise is the
//! spec's, which governance amends. The question has been written down for it.
//!
//! The block is shared across the two adapters, so that one number never means two things
//! *here*: `300` `BadPrice`, `301` `NotNewer`, `302` `FromTheFuture`, `303` `BadDecimals`.
//! `303` is raised by both adapters' constructors; the first three are this adapter's alone.

// The SDK's `contracterror` macro generates items without doc comments, which the workspace's
// `missing_docs = "deny"` would reject; the variants below are documented one by one.
#![allow(missing_docs)]

use soroban_sdk::contracterror;

/// What `post` and the constructor refuse.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `post` was given a price at or below zero. Not a judgement about the value: a price
    /// that is not positive is not a price, and the pool would refuse it anyway (E-003 §3.2).
    /// Refusing here keeps it out of storage, where it would sit as the newest reading and
    /// block every later post of the same second.
    BadPrice = 300,
    /// `post` was given a timestamp older than the stored one. The adapter holds the newest
    /// publication, and accepting an older time would let a keeper that is looping on a cached
    /// read overwrite a fresher price with a staler one. An equal timestamp is not this error
    /// since the amendment of 2026-09-16 (spec §3.3): a corrected price under an unchanged
    /// publication time is something the issuer said, and only the keeper can tell whether it
    /// is worth posting.
    NotNewer = 301,
    /// `post` was given a timestamp later than the ledger's. A publication time in the future
    /// is not a publication time; it would also poison the entry, because no honest later
    /// reading could then be newer.
    FromTheFuture = 302,
    /// The constructor was given more than 18 decimals, which the pool refuses (E-003 §3.2),
    /// so the adapter could only ever be a source the pool skips.
    BadDecimals = 303,
}

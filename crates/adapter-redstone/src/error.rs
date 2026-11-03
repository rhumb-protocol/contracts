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
//! This adapter raises only `303`; the first three belong to `adapter-issuer`, whose `post`
//! can be given a bad value, and the numbers are left free here.
//!
//! There is no error for "this instance holds nothing". `observe` and the views answer `None`
//! in that case, which is the answer a pool can act on: a source without an observation
//! (E-007 §3.2). In practice the branch is unreachable on a live network -- when an instance
//! entry expires the contract instance is archived and every invocation fails before the body
//! runs -- but it is the safe answer if it is ever reached, and spec §6 asks that nothing
//! beyond the errors of §3.3 is exported, so a fifth code is not the way to say it.

// The SDK's `contracttype`, `contracterror`, `contractclient` and `contractimpl` macros
// generate items without doc comments, which the workspace's `missing_docs = "deny"` would
// reject; the items this file writes by hand are documented above each of them.
#![allow(missing_docs)]

use soroban_sdk::contracterror;

/// The error this adapter can return.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The feed's `decimals()` is above the 18 the pool accepts (E-003 §3.2), so a pool could
    /// never accept this source; the constructor refuses rather than leave a dead adapter.
    BadDecimals = 303,
}

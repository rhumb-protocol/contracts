//! The events of spec A-001 §3.3, one `contractevent` each.
//!
//! Every state change this contract makes emits one, because the keeper is unattended and its
//! log is the only other record of what it did: a post that the keeper believes went through
//! and that is not in the chain's events did not happen, and a rotation nobody watched is how
//! a stolen key stays useful.
//!
//! Provenance: nothing here is ported. The event encoding follows the Soroban SDK's
//! `contractevent` macro (the fixed topic first, then the `#[topic]` fields in order; the
//! data is a map from field name to value).

// The SDK's `contractevent` macro generates items without doc comments, which the workspace's
// `missing_docs = "deny"` would reject; the fields below are documented one by one.
#![allow(missing_docs)]

use soroban_sdk::{contractevent, Address};

/// `("init")`: the two roles and the scale the constructor fixed (spec §3.3, as amended),
/// so that a watcher that follows the roles through `keeper` and `governance` starts from the
/// ones the deployment stored rather than from nothing.
#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Init {
    pub governance: Address,
    pub keeper: Address,
    pub decimals: u32,
}

/// `("post", keeper)`: the price and the issuer's publication time, exactly as stored.
#[contractevent(topics = ["post"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Post {
    #[topic]
    pub keeper: Address,
    pub price: i128,
    pub timestamp: u64,
}

/// `("keeper")`: the rotation of the operational key, with both sides of it, so that a watcher
/// can tell a planned rotation from one it did not order.
#[contractevent(topics = ["keeper"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeeperChanged {
    pub governance: Address,
    pub old: Address,
    pub new: Address,
}

/// `("governance")`: the handover of the governance role, with both sides of it.
#[contractevent(topics = ["governance"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceChanged {
    pub old: Address,
    pub new: Address,
}

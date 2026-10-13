//! The one event of spec A-001 §3.2, published by the constructor.
//!
//! The adapter fixes a feed and an asset for the life of the contract and writes nothing after,
//! so the deployment is the one moment there is anything to say: a watcher
//! that reads events learns from `init` which feed and which asset an adapter reads and from
//! which ledger on, without calling the views.
//!
//! Provenance: nothing here is ported. The event encoding follows the Soroban SDK's
//! `contractevent` macro (the fixed topic first; the data is a map from field name to value).

// The SDK's `contractevent` macro generates items without doc comments, which the workspace's
// `missing_docs = "deny"` would reject; the fields below are documented one by one.
#![allow(missing_docs)]

use crate::sep40::Asset;
use soroban_sdk::{contractevent, Address};

/// `("init")`: the SEP-40 contract the adapter reads and the asset it asks that contract for,
/// exactly as stored.
#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Init {
    pub oracle: Address,
    pub asset: Asset,
}

//! What the adapter stores (spec A-001 §3.3).

// The SDK's `contracttype` macro generates items without doc comments, which the workspace's
// `missing_docs = "deny"` would reject; the items below are documented one by one.
#![allow(missing_docs)]

use soroban_sdk::contracttype;

/// The instance storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// The address that may replace the keeper and hand its own role on.
    Governance,
    /// The address that may `post`, and nothing else.
    Keeper,
    /// The scale every posted price is at, fixed by the constructor.
    Decimals,
    /// The newest posted `(price, timestamp)`; absent before the first post.
    Latest,
}

/// The newest posting: the price at the adapter's scale, and the issuer's own publication
/// time in seconds, not the time the keeper read it and not the NAV's valuation time (spec
/// §3.3). Nothing on chain can check which of the three a keeper sent; the keeper's readers
/// carry that rule and its tests hold them to it (spec §4 test 5).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Posting {
    pub price: i128,
    pub timestamp: u64,
}

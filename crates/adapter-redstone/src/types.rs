//! What the adapter stores (spec A-001 §3.2): the feed it reads and the asset it reads there,
//! both written once by the constructor and never again.

// The SDK's `contracttype`, `contracterror`, `contractclient` and `contractimpl` macros
// generate items without doc comments, which the workspace's `missing_docs = "deny"` would
// reject; the items this file writes by hand are documented above each of them.
#![allow(missing_docs)]

use soroban_sdk::contracttype;

/// The instance storage keys. There is no key for a price: this adapter stores no reading,
/// it forwards the feed's (spec §3.2).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// The SEP-40 contract this adapter reads.
    Oracle,
    /// The asset it asks that contract for.
    Asset,
}

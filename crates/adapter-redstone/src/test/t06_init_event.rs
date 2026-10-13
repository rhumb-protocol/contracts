//! The `init` event, decoded from the XDR the host recorded, and the `VERSION` it came with.
//!
//! Spec A-001 §3.2 as amended on 2026-09-29: the constructor publishes
//! `init` with `oracle` and `asset`, so that a watcher that reads events has the adapter's
//! first ledger and what it fixed. The documented shape (`events.rs`): `("init")` with
//! `{oracle, asset}`, an address and a SEP-40 `Asset`. The raw event is read without the event
//! type's own encoder in the loop, because that is what a watcher decodes.
//!
//! The constructor stores both with the deployer's authorisation alone (§3.2): the fixture
//! mocks no authorisation, so every test of this crate deploys without one.
//!
//! An event is part of the interface, so the Wasm that publishes it carries `VERSION` 2.

use super::{Fixture, Mode};
use crate::sep40::Asset;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::xdr::{ContractEventBody, ScVal};
use soroban_sdk::{Address, Env, TryFromVal, Val};
use std::collections::BTreeMap;
use std::string::{String, ToString};

/// A symbol as text.
fn symbol(value: &ScVal) -> String {
    match value {
        ScVal::Symbol(s) => s.to_utf8_string_lossy(),
        other => panic!("not a symbol: {other:?}"),
    }
}

/// A value decoded into `T` in `env`.
fn decode<T: TryFromVal<Env, Val>>(env: &Env, value: &ScVal) -> T {
    let val = Val::try_from_val(env, value).expect("the value did not decode");
    T::try_from_val(env, &val)
        .unwrap_or_else(|_| panic!("the value is not of the documented type: {value:?}"))
}

#[test]
fn the_init_event_decodes_into_its_documented_schema() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let events = f.env.events().all().filter_by_contract(&f.adapter);
    let events = events.events();
    assert_eq!(
        events.len(),
        1,
        "the constructor publishes exactly one event"
    );
    let ContractEventBody::V0(body) = &events[0].body;

    assert_eq!(body.topics.len(), 1, "one topic: the name");
    assert_eq!(symbol(&body.topics[0]), "init".to_string());

    let ScVal::Map(Some(entries)) = &body.data else {
        panic!("the data is not a map: {:?}", body.data);
    };
    let map: BTreeMap<String, ScVal> = entries
        .iter()
        .map(|entry| (symbol(&entry.key), entry.val.clone()))
        .collect();
    let keys: std::vec::Vec<&str> = map.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["asset", "oracle"],
        "the data map's keys are not the documented ones"
    );
    assert_eq!(decode::<Address>(&f.env, &map["oracle"]), f.oracle);
    assert_eq!(decode::<Asset>(&f.env, &map["asset"]), f.asset);

    // What the event says is what the contract holds.
    assert_eq!(f.oracle_view(), Some(f.oracle.clone()));
    assert_eq!(f.asset_view(), Some(f.asset.clone()));
}

#[test]
fn the_wasm_that_publishes_init_is_version_2() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    assert_eq!(f.version(), 2);
    assert_eq!(crate::VERSION, 2);
}

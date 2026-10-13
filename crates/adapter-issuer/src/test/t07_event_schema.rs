//! The three events, decoded from the XDR the host recorded against the schema `events.rs`
//! documents, without the event types' own encoders in the loop.
//!
//! The other event tests compare an emitted event with the same struct's `topics()` and
//! `data()`, so a change to a field name, a field's type or the data format would move both
//! sides at once and pass. What an indexer and a watcher of the keeper's key decode (the events
//! of A-001 §3.3) is the raw event: the topic symbols, the addresses in the topics, and a data
//! map whose keys and value types are the documented ones and nothing more. This test reads
//! exactly that.
//!
//! The documented shapes (`events.rs`): `("post", keeper)` with `{price: i128, timestamp:
//! u64}`; `("keeper")` with `{governance, old, new}`, three addresses; `("governance")` with
//! `{old, new}`, two addresses.

use super::harness::{Fixture, JAN_1_2026};
use super::Mode;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::xdr::{ContractEventBody, ScVal, VecM};
use soroban_sdk::{Address, TryFromVal, Val};
use std::collections::BTreeMap;
use std::string::{String, ToString};

/// A price of one, at 18 decimals.
const ONE: i128 = 1_000_000_000_000_000_000;

/// The one event the adapter published in the last top-level call: its topics and its data,
/// as the XDR the chain carries.
fn the_one_event(f: &Fixture) -> (VecM<ScVal>, ScVal) {
    let events = f.env.events().all().filter_by_contract(&f.adapter);
    let events = events.events();
    assert_eq!(events.len(), 1, "exactly one event was expected");
    let ContractEventBody::V0(body) = &events[0].body;
    (body.topics.clone(), body.data.clone())
}

/// A topic or a key that must be a symbol, as text.
fn symbol(value: &ScVal) -> String {
    match value {
        ScVal::Symbol(s) => s.to_utf8_string_lossy(),
        other => panic!("not a symbol: {other:?}"),
    }
}

/// A value that must be an address.
fn address(f: &Fixture, value: &ScVal) -> Address {
    let val = Val::try_from_val(&f.env, value).expect("the value did not decode");
    Address::try_from_val(&f.env, &val).expect("the value is not an address")
}

/// The data of an event as the map the documentation promises, keyed by the symbols' text,
/// with exactly `keys` and nothing else.
fn data_map(data: &ScVal, keys: &[&str]) -> BTreeMap<String, ScVal> {
    let ScVal::Map(Some(entries)) = data else {
        panic!("the data is not a map: {data:?}");
    };
    let map: BTreeMap<String, ScVal> = entries
        .iter()
        .map(|entry| (symbol(&entry.key), entry.val.clone()))
        .collect();
    let mut found: std::vec::Vec<&str> = map.keys().map(String::as_str).collect();
    found.sort_unstable();
    let mut want: std::vec::Vec<&str> = keys.to_vec();
    want.sort_unstable();
    assert_eq!(
        found, want,
        "the data map's keys are not the documented ones"
    );
    map
}

#[test]
fn the_post_event_decodes_into_its_documented_schema() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let at = JAN_1_2026 - 60;
    assert_eq!(f.post(ONE, at), Ok(()));
    let (topics, data) = the_one_event(&f);

    assert_eq!(topics.len(), 2, "two topics: the name and the keeper");
    assert_eq!(symbol(&topics[0]), "post".to_string());
    assert_eq!(address(&f, &topics[1]), f.keeper);

    let map = data_map(&data, &["price", "timestamp"]);
    let price = i128::try_from_val(
        &f.env,
        &Val::try_from_val(&f.env, &map["price"]).expect("price did not decode"),
    )
    .expect("price is not an i128");
    let timestamp = u64::try_from_val(
        &f.env,
        &Val::try_from_val(&f.env, &map["timestamp"]).expect("timestamp did not decode"),
    )
    .expect("timestamp is not a u64");
    assert_eq!((price, timestamp), (ONE, at));
    // What the event says is what the contract now holds.
    assert_eq!(f.observe(), Some((price, 18, timestamp)));
}

#[test]
fn the_keeper_event_decodes_into_its_documented_schema() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_keeper(&new), Ok(()));
    let (topics, data) = the_one_event(&f);

    assert_eq!(topics.len(), 1, "one topic: the name");
    assert_eq!(symbol(&topics[0]), "keeper".to_string());

    let map = data_map(&data, &["governance", "old", "new"]);
    assert_eq!(address(&f, &map["governance"]), f.governance);
    assert_eq!(address(&f, &map["old"]), f.keeper);
    assert_eq!(address(&f, &map["new"]), new);
    assert_eq!(f.keeper_view(), Some(address(&f, &map["new"])));
}

#[test]
fn the_governance_event_decodes_into_its_documented_schema() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_governance(&new), Ok(()));
    let (topics, data) = the_one_event(&f);

    assert_eq!(topics.len(), 1, "one topic: the name");
    assert_eq!(symbol(&topics[0]), "governance".to_string());

    let map = data_map(&data, &["old", "new"]);
    assert_eq!(address(&f, &map["old"]), f.governance);
    assert_eq!(address(&f, &map["new"]), new);
    assert_eq!(f.governance_view(), Some(address(&f, &map["new"])));
}

//! The `init` event, decoded from the XDR the host recorded, and the `VERSION` it came with.
//!
//! Spec A-001 §3.3 as amended on 2026-09-29: the constructor publishes
//! `init` with `governance`, `keeper` and `decimals`, so that a watcher that reconstructs the
//! roles from the adapter's events starts from the ones the deployment fixed. The documented
//! shape (`events.rs`): `("init")` with `{governance, keeper, decimals}`, two addresses and a
//! `u32`. As in `t07_event_schema`, the raw event is read without the event type's own encoder
//! in the loop, because that is what the indexer and a watcher decode.
//!
//! An event is part of the interface, so the Wasm that publishes it carries `VERSION` 2.

use super::harness::{adapter_wasm_for_tests, test_env};
use soroban_sdk::testutils::{Address as _, Events as _};
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
    // Deployed here rather than through the fixture, so that nothing runs between the
    // deployment and the reading of its events.
    let Some(wasm) = adapter_wasm_for_tests() else {
        return;
    };
    let env = test_env();
    let governance = Address::generate(&env);
    let keeper = Address::generate(&env);
    let adapter = env.register(wasm.as_slice(), (governance.clone(), keeper.clone(), 7u32));

    let events = env.events().all().filter_by_contract(&adapter);
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
        ["decimals", "governance", "keeper"],
        "the data map's keys are not the documented ones"
    );
    assert_eq!(decode::<Address>(&env, &map["governance"]), governance);
    assert_eq!(decode::<Address>(&env, &map["keeper"]), keeper);
    assert_eq!(decode::<u32>(&env, &map["decimals"]), 7);

    // What the event says is what the contract holds.
    let client = super::IssuerClient::new(&env, &adapter);
    assert_eq!(client.governance(), Some(governance));
    assert_eq!(client.keeper(), Some(keeper));
    assert_eq!(client.decimals(), Some(7));
}

#[test]
fn the_wasm_that_publishes_init_is_version_2() {
    let Some(wasm) = adapter_wasm_for_tests() else {
        return;
    };
    let env = test_env();
    let adapter = env.register(
        wasm.as_slice(),
        (Address::generate(&env), Address::generate(&env), 18u32),
    );
    assert_eq!(super::IssuerClient::new(&env, &adapter).version(), 2);
    assert_eq!(crate::VERSION, 2);
}

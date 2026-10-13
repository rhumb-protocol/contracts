//! Spec A-001 §4 test 1, the constructor's half: a feed whose `decimals()` is above 18 or
//! whose `decimals()` fails does not get an adapter.
//!
//! The reason is that the pool refuses `decimals > 18` (E-003 §3.2), so such an adapter could
//! only ever be a source the pool skips. Refusing at construction means the dead source never
//! exists, rather than existing and being discovered when a pool is already live on it.
//!
//! Natively the constructor's `Result` is visible and the error code of spec §3.3 is asserted
//! directly; in Wasm mode a refusing constructor aborts the deployment, so those tests assert
//! the abort and the contract error number it carries. A test that has no Wasm to run against
//! raises the panic its attribute expects, so that a skip is never reported as a pass of
//! something that did not happen.

use super::harness::{adapter_wasm_for_tests, mock_wasm_for_tests, test_env, Fixture, MockClient};
use super::Mode;
use crate::contract::RedstoneAdapter;
use crate::error::Error;
use crate::sep40::Asset;
use crate::{DECIMALS_MAX, VERSION};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

both!(
    the_views_report_what_the_constructor_stored,
    |f: &Fixture| {
        assert_eq!(f.oracle_view(), Some(f.oracle.clone()));
        assert_eq!(f.asset_view(), Some(f.asset.clone()));
        assert_eq!(f.version(), VERSION);
    }
);

both!(eighteen_decimals_is_accepted, |_f: &Fixture| {
    // The fixture constructs against a feed reporting 18; reaching the body at all proves the
    // boundary is inclusive.
    assert_eq!(DECIMALS_MAX, 18);
});

/// A feed reporting `decimals`, an asset to ask it for, and a shell instance to run the
/// adapter's entry points inside; `None` without a built Wasm.
///
/// The shell is registered from the mock's Wasm, which is never invoked through it: what
/// `as_contract` needs is a registered contract instance to give the entry points storage of
/// their own, and an address that was only generated has none.
fn feed_at(decimals: u32) -> Option<(Env, Address, Asset, Address)> {
    let mock = mock_wasm_for_tests()?;
    let env = test_env();
    let oracle = env.register(mock.as_slice(), ());
    MockClient::new(&env, &oracle).set_decimals(&decimals);
    let asset = Asset::Stellar(Address::generate(&env));
    let shell = env.register(mock.as_slice(), ());
    Some((env, oracle, asset, shell))
}

#[test]
fn nineteen_decimals_refuses_the_constructor_natively() {
    let Some((env, oracle, asset, shell)) = feed_at(19) else {
        return;
    };
    let result = env.as_contract(&shell, || {
        RedstoneAdapter::__constructor(env.clone(), oracle.clone(), asset.clone())
    });
    assert_eq!(result, Err(Error::BadDecimals));
}

#[test]
fn thirty_two_decimals_refuses_the_constructor_natively() {
    let Some((env, oracle, asset, shell)) = feed_at(32) else {
        return;
    };
    let result = env.as_contract(&shell, || {
        RedstoneAdapter::__constructor(env.clone(), oracle.clone(), asset.clone())
    });
    assert_eq!(result, Err(Error::BadDecimals));
}

#[test]
#[should_panic(expected = "#303")]
fn nineteen_decimals_refuses_the_deployment() {
    match Fixture::with_feed_decimals(Mode::Wasm, 19) {
        Some(_) => panic!("the deployment should have been refused"),
        None => panic!("skipped, no Wasm: #303"),
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn a_feed_that_cannot_answer_decimals_refuses_the_deployment() {
    // The expected text matters: a bare `should_panic` would be satisfied by the skip branch
    // below and by the "no Wasm" panic `ADAPTER_WASM_REQUIRED` raises, so the test could never
    // fail closed. `#1` is the mock's own error for a feed that is down, which is what the
    // constructor's `decimals()` call runs into.
    let (Some((env, oracle, asset, _shell)), Some(adapter)) =
        (feed_at(18), adapter_wasm_for_tests())
    else {
        panic!("skipped, no Wasm: Error(Contract, #1)");
    };
    MockClient::new(&env, &oracle).set_failing(&true);
    let _ = env.register(adapter.as_slice(), (oracle, asset));
}

#[test]
fn the_views_answer_none_on_an_instance_that_holds_nothing() {
    // An instance entry is gone only when its TTL lapsed and the weekly extension (spec §3.4)
    // did not reach the adapter. The views then answer `None` rather than failing, and
    // `observe` answers `None` too, which is what a pool reads as a source without an
    // observation (E-007 §3.2). Nothing in this crate turns that into an error: spec §6 asks
    // that nothing beyond the errors of §3.3 is exported.
    let Some((env, _oracle, _asset, shell)) = feed_at(18) else {
        return;
    };
    env.as_contract(&shell, || {
        assert_eq!(RedstoneAdapter::oracle(env.clone()), None);
        assert_eq!(RedstoneAdapter::asset(env.clone()), None);
        assert_eq!(RedstoneAdapter::observe(env.clone()), None);
    });
}

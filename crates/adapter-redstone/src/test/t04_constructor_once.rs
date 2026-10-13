//! The constructor runs once, at deployment, and never again: a second call naming another
//! feed or another asset is refused by the host and changes nothing.
//!
//! Why this is pinned although the contract has no setter: the feed and the asset are fixed by
//! `__constructor` and nowhere else, so the one way to point a live adapter at another feed
//! without governance's `set_rate` on the pool would be to run its constructor a second time.
//! Soroban exposes the constructor to the deployment only and refuses a direct invocation of
//! it; this test holds the deployed Wasm to that.

use super::{Fixture, Mode};
use crate::sep40::Asset;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, IntoVal, Symbol, Val, Vec};

#[test]
fn the_constructor_cannot_run_a_second_time_on_a_deployed_adapter() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    // A second feed that answers, so that the refusal cannot be the feed's `decimals()` failing.
    let other_oracle = f.oracle.clone();
    let other_asset = Asset::Stellar(Address::generate(&f.env));
    let args: Vec<Val> = Vec::from_array(
        &f.env,
        [
            other_oracle.into_val(&f.env),
            other_asset.clone().into_val(&f.env),
        ],
    );
    let result = f.env.try_invoke_contract::<(), soroban_sdk::Error>(
        &f.adapter,
        &Symbol::new(&f.env, "__constructor"),
        args,
    );
    assert!(
        result.is_err(),
        "a second run of the constructor was accepted: {result:?}"
    );
    assert_eq!(f.oracle_view(), Some(f.oracle.clone()));
    assert_eq!(f.asset_view(), Some(f.asset.clone()));
    assert_ne!(f.asset_view(), Some(other_asset));
}

//! The constructor runs once, at deployment, and never again: a second call naming other
//! roles is refused by the host and changes nothing.
//!
//! Why this is pinned although no entry point of the contract offers it: the roles are stored
//! by `__constructor` and nowhere else without an authorisation, so the one way to take an
//! adapter over without a key would be to run its constructor a second time. Soroban exposes
//! the constructor to the deployment only and refuses a direct invocation of it; this test
//! holds the deployed Wasm to that, so that a change of the SDK, of the host or of the
//! function's name that made it callable would show up here and not on a chain.

use super::harness::Fixture;
use super::Mode;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, IntoVal, Symbol, Val, Vec};

#[test]
fn the_constructor_cannot_run_a_second_time_on_a_deployed_adapter() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let other_governance = Address::generate(&f.env);
    let other_keeper = Address::generate(&f.env);
    let args: Vec<Val> = Vec::from_array(
        &f.env,
        [
            other_governance.into_val(&f.env),
            other_keeper.into_val(&f.env),
            6u32.into_val(&f.env),
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
    // The roles and the scale are what the deployment fixed.
    assert_eq!(f.governance_view(), Some(f.governance.clone()));
    assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
    assert_eq!(f.decimals_view(), Some(18));
}

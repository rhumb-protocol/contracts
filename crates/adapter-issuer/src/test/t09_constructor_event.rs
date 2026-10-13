//! The constructor stores the two roles and the scale and publishes nothing.
//!
//! Every later change of a role emits an event with both sides of it (`keeper`,
//! `governance`), so that a watcher can follow the roles; but the first roles, the ones the
//! deployment fixed, are visible only in the deployment itself and through the views. A
//! watcher that reconstructs the roles from the adapter's events alone starts from nothing.
//! The engine's pool publishes `init` with its roles at creation (E-007 §3.5); spec A-001 §3.3
//! lists no such event for the adapter, so this test states the rule the engine follows and
//! fails on the contract as it is. Whether the rule is wanted is a question for the spec.

use super::harness::{adapter_wasm_for_tests, test_env};
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::Address;

#[test]
fn the_deployment_publishes_the_roles_it_fixed() {
    // Deployed here rather than through the fixture, so that nothing runs between the
    // deployment and the reading of its events.
    let Some(wasm) = adapter_wasm_for_tests() else {
        return;
    };
    let env = test_env();
    env.mock_all_auths();
    let governance = Address::generate(&env);
    let keeper = Address::generate(&env);
    let adapter = env.register(wasm.as_slice(), (governance, keeper, 18u32));
    let events = env.events().all().filter_by_contract(&adapter);
    assert_eq!(
        events.events().len(),
        1,
        "the constructor fixed two roles and a scale and published no event"
    );
}

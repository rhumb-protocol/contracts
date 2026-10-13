//! The constructor stores governance and the keeper with the deployer's authorisation alone.
//!
//! Finding 9 of the exploit pass over this repository asked whether the addresses a deployment
//! stores as its roles should have to sign for it, as the Blend v1 assessment asked of a
//! pool's initialiser; its first form of this test demanded their signatures and failed. Spec
//! A-001 §3.3 as amended on 2026-09-29 rules the other way, the rule the engine follows for
//! the pool and the router too: the constructor requires neither role's
//! signature, anyone can deploy an adapter naming the brand's addresses, and such an adapter
//! is not the brand's. The brand records its own deployments and reads both roles back after
//! every one, and a look-alike reaches no pool unless governance signs it into one. This test
//! is that rule: a later `require_auth` of either role in the constructor would make every
//! deployment by the deploy key fail, and fails here first.

use super::harness::{adapter_wasm_for_tests, test_env};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Address;

#[test]
fn a_deployment_naming_governance_and_the_keeper_needs_neither_signature() {
    let Some(wasm) = adapter_wasm_for_tests() else {
        return;
    };
    let env = test_env();
    // No authorisation is mocked: a `require_auth` of either role inside the constructor would
    // fail the deployment.
    let governance = Address::generate(&env);
    let keeper = Address::generate(&env);
    let adapter = env.register(wasm.as_slice(), (governance.clone(), keeper.clone(), 18u32));
    let client = super::IssuerClient::new(&env, &adapter);
    assert_eq!(client.governance(), Some(governance));
    assert_eq!(client.keeper(), Some(keeper));
}

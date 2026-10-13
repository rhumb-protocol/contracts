//! The constructor stores the feed and the asset and publishes nothing.
//!
//! The two are fixed for the life of the contract, so there is no later event that could
//! carry them; a watcher of the chain learns which feed and which asset an adapter reads only
//! from the deployment itself or by calling the views. The engine's pool publishes `init`
//! with what its constructor fixed (E-007 §3.5); spec A-001 §3.2 lists no event for this
//! adapter at all, so this test states the rule the engine follows and fails on the contract
//! as it is. Whether the rule is wanted is a question for the spec.

use super::{Fixture, Mode};
use soroban_sdk::testutils::Events as _;

#[test]
fn the_deployment_publishes_the_feed_and_the_asset_it_fixed() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let events = f.env.events().all().filter_by_contract(&f.adapter);
    assert_eq!(
        events.events().len(),
        1,
        "the constructor fixed a feed and an asset and published no event"
    );
}

//! Spec A-001 §4 test 1: `observe` returns exactly what the feed returns, for every value the
//! feed can give; a `None` from `lastprice` is a `None` from the adapter; a feed call that
//! fails fails `observe`.
//!
//! "Exactly" is the whole point of this adapter, so the property test covers the full ranges
//! the spec names -- every price in `i128`, every `decimals` in `0 ..= 32`, every timestamp in
//! `u64` -- and not only the values a pool would accept. Values the pool refuses must still
//! arrive at the pool unchanged, because the refusal is the pool's to make (E-003 §3.2) and an
//! adapter that quietly dropped them would hide a misbehaving feed.

use super::{Fixture, Mode};
use proptest::prelude::*;

both!(none_before_the_feed_has_a_price, |f: &Fixture| {
    f.feed().clear_price();
    assert_eq!(f.observe(), None);
});

both!(a_reading_comes_back_unchanged, |f: &Fixture| {
    f.feed()
        .set_price(&1_000_000_000_000_000_000, &1_757_000_000);
    assert_eq!(
        f.observe(),
        Some((1_000_000_000_000_000_000, 18, 1_757_000_000))
    );
});

both!(
    the_feeds_decimals_are_reported_not_the_ones_at_deploy,
    |f: &Fixture| {
        // A feed may change its scale after the adapter was deployed. The adapter reports what the
        // feed says now, never what it said at construction.
        f.feed().set_price(&123_456, &1_757_000_000);
        f.feed().set_decimals(&6);
        assert_eq!(f.observe(), Some((123_456, 6, 1_757_000_000)));
    }
);

both!(
    a_negative_price_is_forwarded_not_swallowed,
    |f: &Fixture| {
        // The pool refuses `price <= 0` (E-003 §3.2). The adapter must still hand it over, so that
        // the pool's `refresh` event reports the source as ignored rather than as silent.
        f.feed().set_price(&-1, &1_757_000_000);
        assert_eq!(f.observe(), Some((-1, 18, 1_757_000_000)));
    }
);

both!(a_zero_price_is_forwarded, |f: &Fixture| {
    f.feed().set_price(&0, &1_757_000_000);
    assert_eq!(f.observe(), Some((0, 18, 1_757_000_000)));
});

both!(
    a_timestamp_beyond_the_pools_bound_is_forwarded,
    |f: &Fixture| {
        // The pool refuses a timestamp above 2^40 (E-003 §3.2). The adapter does not: a feed that
        // publishes milliseconds would be visible in the pool's event, not hidden here.
        f.feed().set_price(&1, &u64::MAX);
        assert_eq!(f.observe(), Some((1, 18, u64::MAX)));
    }
);

both!(extreme_values_survive_the_boundary, |f: &Fixture| {
    for price in [i128::MIN, i128::MAX, -1, 0, 1] {
        for timestamp in [0u64, 1, u64::MAX] {
            f.feed().set_price(&price, &timestamp);
            assert_eq!(f.observe(), Some((price, 18, timestamp)));
        }
    }
});

#[test]
fn a_failing_feed_fails_observe_wasm() {
    if let Some(f) = Fixture::new(Mode::Wasm) {
        f.feed().set_price(&1, &1);
        f.feed().set_failing(&true);
        let client = super::AdapterClient::new(&f.env, &f.adapter);
        assert!(
            client.try_observe().is_err(),
            "a feed that is down must fail observe, so that the pool sees a dead source"
        );
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn a_failing_feed_fails_observe_native() {
    if let Some(f) = Fixture::new(Mode::Native) {
        f.feed().set_price(&1, &1);
        f.feed().set_failing(&true);
        let _ = f.observe();
    } else {
        // Nothing to run; the panic the attribute expects is raised so that the skip is not
        // reported as a failure.
        soroban_sdk::panic_with_error!(
            &soroban_sdk::Env::default(),
            soroban_sdk::Error::from_contract_error(1)
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Every price, every scale the spec names and every timestamp, unchanged through the
    /// contract boundary.
    #[test]
    fn observe_forwards_every_reading(
        price in any::<i128>(),
        decimals in 0u32..=32,
        timestamp in any::<u64>(),
    ) {
        if let Some(f) = Fixture::new(Mode::Wasm) {
            f.feed().set_decimals(&decimals);
            f.feed().set_price(&price, &timestamp);
            prop_assert_eq!(f.observe(), Some((price, decimals, timestamp)));
        }
    }
}

/// Spec A-001 §4 test 1, the part the mock could not show until it answered per asset: the
/// adapter asks the feed for the asset its constructor stored, and for no other.
///
/// On a real feed the asset argument selects the feed. An adapter that asked for the wrong one
/// would be handed another token's price, at a plausible scale and a valid timestamp, and the
/// pool would accept it: a USDY pool quoting against USDC's price, with nothing anywhere
/// reporting a fault. Every other test in this file would pass through that bug, because they
/// each register one asset and would be answered whichever asset was asked for. This one
/// registers two, and does it in both orders, so that it cannot pass on an adapter that always
/// takes whichever reading was stored first.
mod the_adapter_asks_for_its_own_asset {
    use super::*;
    use crate::sep40::Asset;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Address;

    /// A price of one, at 18 decimals, and a price of two.
    const ONE: i128 = 1_000_000_000_000_000_000;
    const TWO: i128 = 2_000_000_000_000_000_000;

    /// Registers two assets on the fixture's feed and asserts the adapter is handed its own.
    /// `mine_first` decides which of the two is stored first.
    fn check(mode: Mode, mine_first: bool) {
        let Some(f) = Fixture::new(mode) else {
            return;
        };
        // A second asset on the same feed, in this fixture's own environment.
        let other = Asset::Stellar(Address::generate(&f.env));
        if mine_first {
            f.feed().set_price_for(&f.asset, &ONE, &1_757_000_001);
            f.feed().set_price_for(&other, &TWO, &1_757_000_002);
        } else {
            f.feed().set_price_for(&other, &TWO, &1_757_000_002);
            f.feed().set_price_for(&f.asset, &ONE, &1_757_000_001);
        }
        // No reading for every asset: whatever comes back was selected by the argument.
        f.feed().clear_price();

        assert_eq!(
            f.observe(),
            Some((ONE, 18, 1_757_000_001)),
            "the adapter was handed the other asset's reading"
        );
    }

    #[test]
    fn wasm_with_its_own_asset_stored_first() {
        check(Mode::Wasm, true);
    }

    #[test]
    fn wasm_with_the_other_asset_stored_first() {
        check(Mode::Wasm, false);
    }

    #[test]
    fn native_with_its_own_asset_stored_first() {
        check(Mode::Native, true);
    }

    #[test]
    fn native_with_the_other_asset_stored_first() {
        check(Mode::Native, false);
    }
}

#[test]
fn an_asset_the_feed_does_not_carry_is_none_not_another_assets_price() {
    // The failure mode this guards: a feed that dropped an asset must make the source go
    // quiet, never make it quietly answer with a neighbour's price.
    use soroban_sdk::testutils::Address as _;
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let other = crate::sep40::Asset::Stellar(soroban_sdk::Address::generate(&f.env));
    f.feed()
        .set_price_for(&other, &1_000_000_000_000_000_000, &1_757_000_002);
    f.feed().clear_price();
    assert_eq!(f.observe(), None);
}

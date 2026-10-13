//! Spec A-001 §4 test 2, the posting half: what `post` stores, what it emits, what it refuses,
//! and what `observe` says before and after.

use super::harness::{adapter_wasm_for_tests, test_env, Fixture, JAN_1_2026};
use super::Mode;
use crate::contract::IssuerAdapter;
use crate::error::Error;
use crate::events::Post;
use crate::types::Posting;
use crate::VERSION;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

/// A price of one, at 18 decimals.
const ONE: i128 = 1_000_000_000_000_000_000;

both!(nothing_is_observed_before_the_first_post, |f: &Fixture| {
    assert_eq!(f.latest(), None);
    assert_eq!(f.observe(), None);
});

both!(
    the_constructor_stored_both_roles_and_the_scale,
    |f: &Fixture| {
        assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
        assert_eq!(f.governance_view(), Some(f.governance.clone()));
        assert_eq!(f.decimals_view(), Some(18));
        assert_eq!(f.version(), VERSION);
    }
);

both!(
    a_post_is_stored_and_observed_with_the_scale,
    |f: &Fixture| {
        let at = JAN_1_2026 - 60;
        assert_eq!(f.post(ONE, at), Ok(()));
        assert_eq!(
            f.latest(),
            Some(Posting {
                price: ONE,
                timestamp: at
            })
        );
        assert_eq!(f.observe(), Some((ONE, 18, at)));
    }
);

both!(a_later_post_replaces_the_earlier_one, |f: &Fixture| {
    assert_eq!(f.post(ONE, JAN_1_2026 - 120), Ok(()));
    assert_eq!(f.post(ONE + 1, JAN_1_2026 - 60), Ok(()));
    assert_eq!(f.observe(), Some((ONE + 1, 18, JAN_1_2026 - 60)));
});

both!(a_price_at_or_below_zero_is_refused, |f: &Fixture| {
    assert_eq!(f.post(0, JAN_1_2026 - 60), Err(Error::BadPrice));
    assert_eq!(f.post(-1, JAN_1_2026 - 60), Err(Error::BadPrice));
    assert_eq!(f.post(i128::MIN, JAN_1_2026 - 60), Err(Error::BadPrice));
    // Nothing was stored: a refused post leaves the adapter as it was.
    assert_eq!(f.latest(), None);
    assert_eq!(f.observe(), None);
});

both!(
    a_changed_price_under_the_same_timestamp_is_stored,
    |f: &Fixture| {
        // The bound is "older than the stored one", not "not newer than" (spec §3.3, amended
        // 2026-09-16). An issuer that corrects a price without moving its publication time has
        // said something, and an adapter that turned it away would hold a price the issuer no
        // longer publishes until the issuer's clock happened to move.
        let at = JAN_1_2026 - 60;
        assert_eq!(f.post(ONE, at), Ok(()));
        assert_eq!(f.post(ONE + 1, at), Ok(()));
        assert_eq!(f.observe(), Some((ONE + 1, 18, at)));
    }
);

both!(
    the_same_price_under_the_same_timestamp_is_stored_too,
    |f: &Fixture| {
        // The adapter compares times and not prices: the keeper is what decides whether a reading
        // is worth a fee (A-001 §3.4), and a bound here would be a second, quieter rule about news
        // in the one place that cannot see what the keeper read.
        let at = JAN_1_2026 - 60;
        assert_eq!(f.post(ONE, at), Ok(()));
        assert_eq!(f.post(ONE, at), Ok(()));
        assert_eq!(f.observe(), Some((ONE, 18, at)));
    }
);

both!(an_older_timestamp_is_refused, |f: &Fixture| {
    // What `NotNewer` is left to guard: a keeper looping on a cached read would otherwise
    // overwrite a fresher price with an older one, and the pool would never know.
    let at = JAN_1_2026 - 60;
    assert_eq!(f.post(ONE, at), Ok(()));
    assert_eq!(f.post(ONE + 1, at - 1), Err(Error::NotNewer));
    assert_eq!(f.observe(), Some((ONE, 18, at)));
    // A day older is the same answer, and the first post still stands.
    assert_eq!(f.post(ONE + 1, at - 86_400), Err(Error::NotNewer));
    assert_eq!(f.observe(), Some((ONE, 18, at)));
});

both!(a_timestamp_past_the_ledgers_is_refused, |f: &Fixture| {
    assert_eq!(f.post(ONE, f.now() + 1), Err(Error::FromTheFuture));
    assert_eq!(f.post(ONE, u64::MAX), Err(Error::FromTheFuture));
    assert_eq!(f.latest(), None);
});

both!(the_ledgers_own_timestamp_is_accepted, |f: &Fixture| {
    // The bound is "later than the ledger's", not "at the ledger's": an issuer that published
    // in this very second must not be turned away.
    assert_eq!(f.post(ONE, f.now()), Ok(()));
    assert_eq!(f.observe(), Some((ONE, 18, f.now())));
});

both!(
    a_timestamp_of_zero_is_accepted_and_stays_acceptable,
    |f: &Fixture| {
        // Zero is in the past and nothing is stored yet, so the first post takes it; a second one
        // at zero is not older than zero, so it is taken too (spec §3.3, amended 2026-09-16).
        assert_eq!(f.post(ONE, 0), Ok(()));
        assert_eq!(f.post(ONE + 1, 0), Ok(()));
        assert_eq!(f.observe(), Some((ONE + 1, 18, 0)));
    }
);

#[test]
fn a_post_emits_the_price_and_the_time_it_stored() {
    // The keeper is unattended: what it believes it posted and what the chain recorded are
    // reconciled through this event, so it carries both numbers exactly as stored, and the
    // keeper in a topic so that a watcher can filter on the key that posted.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let at = JAN_1_2026 - 60;
    assert_eq!(f.post(ONE, at), Ok(()));
    f.assert_events(&[&Post {
        keeper: f.keeper.clone(),
        price: ONE,
        timestamp: at,
    }]);
}

#[test]
fn a_refused_post_emits_nothing() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    assert_eq!(f.post(-1, JAN_1_2026 - 60), Err(Error::BadPrice));
    f.assert_events(&[]);
}

/// A registered shell instance to run the constructor inside, and the roles to give it.
///
/// The fixture deploys the Wasm, so the constructor only ever runs inside it and its own
/// `Result` is never visible on the host: spec §6 asks for 100 % line coverage of this crate's
/// source, and those lines are reached only here.
fn shell() -> Option<(Env, Address, Address, Address)> {
    let wasm = adapter_wasm_for_tests()?;
    let env = test_env();
    env.mock_all_auths();
    let governance = Address::generate(&env);
    let keeper = Address::generate(&env);
    // Registered from the adapter's own Wasm with a scale the constructor accepts, so that the
    // instance exists; the native constructor then writes over what the deployment stored.
    let id = env.register(wasm.as_slice(), (governance.clone(), keeper.clone(), 18u32));
    Some((env, id, governance, keeper))
}

#[test]
fn nineteen_decimals_refuses_the_constructor_natively() {
    let Some((env, id, governance, keeper)) = shell() else {
        return;
    };
    let result = env.as_contract(&id, || {
        IssuerAdapter::__constructor(env.clone(), governance.clone(), keeper.clone(), 19)
    });
    assert_eq!(result, Err(Error::BadDecimals));
}

#[test]
fn thirty_two_decimals_refuses_the_constructor_natively() {
    let Some((env, id, governance, keeper)) = shell() else {
        return;
    };
    let result = env.as_contract(&id, || {
        IssuerAdapter::__constructor(env.clone(), governance.clone(), keeper.clone(), 32)
    });
    assert_eq!(result, Err(Error::BadDecimals));
}

#[test]
fn eighteen_decimals_is_accepted_by_the_constructor_natively() {
    // The boundary is inclusive, and the constructor stores what it was given.
    let Some((env, id, governance, keeper)) = shell() else {
        return;
    };
    let result = env.as_contract(&id, || {
        IssuerAdapter::__constructor(env.clone(), governance.clone(), keeper.clone(), 18)
    });
    assert_eq!(result, Ok(()));
    env.as_contract(&id, || {
        assert_eq!(IssuerAdapter::decimals(env.clone()), Some(18));
        assert_eq!(IssuerAdapter::keeper(env.clone()), Some(keeper.clone()));
        assert_eq!(
            IssuerAdapter::governance(env.clone()),
            Some(governance.clone())
        );
        // Nothing is stored for a price: the adapter holds none until the keeper posts.
        assert_eq!(IssuerAdapter::latest(env.clone()), None);
    });
}

#[test]
#[should_panic(expected = "#303")]
fn more_than_eighteen_decimals_refuses_the_deployment() {
    // The Wasm half: a refusing constructor aborts the deployment with the error code of spec
    // §3.3. The expected text matters -- a bare `should_panic` would also be satisfied by the
    // "no Wasm" panic `ADAPTER_WASM_REQUIRED` raises, so the test could never fail closed.
    match Fixture::with_decimals(Mode::Wasm, 19) {
        Some(_) => panic!("19 decimals should have been refused"),
        None => panic!("skipped, no Wasm: #303"),
    }
}

#[test]
fn eighteen_decimals_is_accepted() {
    if let Some(f) = Fixture::with_decimals(Mode::Wasm, 18) {
        assert_eq!(f.decimals_view(), Some(18));
    }
}

#[test]
fn a_coarser_scale_is_reported_as_it_was_created() {
    // Every source of Rhumb is deployed at 18 and the keeper scales to 18 before posting
    // (spec §3.4), but the contract accepts any scale the pool would, and reports it
    // unchanged so that the pool scales the price itself.
    if let Some(f) = Fixture::with_decimals(Mode::Wasm, 6) {
        assert_eq!(f.post(1_000_000, JAN_1_2026 - 60), Ok(()));
        assert_eq!(f.observe(), Some((1_000_000, 6, JAN_1_2026 - 60)));
    }
}

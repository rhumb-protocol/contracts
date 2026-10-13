//! The scenarios of E-003 test 7, as spec A-001 §4 test 4 lists them: one source alone never
//! moves the rate; two agreeing sources set the interval; a post that disagrees by more than
//! `cross_check` suspends; a dead adapter drops out and the rate is lost after the grace
//! window; a `None` from the feed is skipped and reported.
//!
//! Every one of them runs through the pool: the adapters are never called directly here. What
//! is asserted is therefore not what an adapter returns -- the other crates' tests do that --
//! but what the engine did with it, read back from the pool's own rate state and its events.

use super::harness::{Fixture, BP, ISSUER, ONE, REDSTONE};
use crate::abi::{code, Direction, RateStatus, SourceOutcome, SuspendReason};
use crate::rate_bytes::Status;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::Address;

/// One token, in the tokens' own seven decimals.
const ONE_TOKEN: i128 = 10_000_000;

/// The liquidity every scenario that quotes deposits: a thousand of each token, which is well
/// inside the cap of A-001 §7.4.
const LIQUIDITY: i128 = 1_000 * ONE_TOKEN;

/// A quote that fails, with the contract error code the host returned.
fn quote_error(f: &Fixture) -> u32 {
    match f.pool().try_quote_exact_in(&Direction::AssetIn, &ONE_TOKEN) {
        Ok(Ok(amount)) => panic!("the pool quoted {amount} where it should have refused"),
        Ok(Err(e)) => panic!("the return value did not decode: {e:?}"),
        Err(Ok(e)) => {
            assert!(
                e.is_type(soroban_sdk::xdr::ScErrorType::Contract),
                "not a contract error: {e:?}"
            );
            e.get_code()
        }
        Err(Err(e)) => panic!("the host refused the call: {e:?}"),
    }
}

/// The pool reads both sources and the issuer's post is the only reading there is: the
/// observation is accepted and stored, and the interval stays exactly where the genesis put
/// it. A second source has not confirmed anything, so there is nothing to move to -- which is
/// the whole reason two sources are configured (E-003 §3.2 gate 6, "pending").
#[test]
fn one_source_alone_never_moves_the_rate() {
    let Some(f) = Fixture::new() else { return };
    let before = f.rate_state();
    assert_eq!((before.low, before.high), (ONE, ONE));

    f.advance(100);
    f.post(ONE + 2 * BP, f.now());
    let change = f.refresh();

    assert_eq!(change.before, RateStatus::Active);
    assert_eq!(change.after, RateStatus::Active);
    let after = f.rate_state();
    assert_eq!(
        (after.low, after.high, after.updated_at),
        (before.low, before.high, before.updated_at),
        "one source moved the interval"
    );
    let stored = after.last_by_source[ISSUER].expect("the issuer's observation was not stored");
    assert_eq!((stored.rate, stored.at), (ONE + 2 * BP, f.now()));
    assert!(after.last_by_source[REDSTONE].is_none());
}

/// Both sources publish within the cross-check: the interval becomes the two rates and carries
/// the newer of the two publication times.
#[test]
fn two_agreeing_sources_set_the_interval() {
    let Some(f) = Fixture::new() else { return };
    f.advance(100);
    let issuer_at = f.now();
    f.post(ONE + 2 * BP, issuer_at);
    f.refresh();

    f.advance(100);
    let feed_at = f.now();
    f.publish_feed(f.at_feed_scale(ONE + 4 * BP), feed_at);
    let change = f.refresh();

    assert_eq!(change.after, RateStatus::Active);
    let state = f.rate_state();
    assert_eq!((state.low, state.high), (ONE + 2 * BP, ONE + 4 * BP));
    assert_eq!(state.updated_at, feed_at, "the newer publication time");
    assert_eq!(state.accepted_at, f.now());
    assert_eq!(
        f.last_refresh_outcomes(),
        std::vec![SourceOutcome::Accepted, SourceOutcome::NotNewer],
        "the feed was accepted; the issuer's reading is now older than the interval"
    );
}

/// A keeper post that sits further from the live feed than `cross_check` suspends the rate
/// with `SourcesDisagree`, keeps the last accepted interval, and stops the pool trading.
///
/// The post is inside every other gate: nine basis points against a ten-point step bound, and
/// a day and a half of drift allowance against a five-point band plus a five-point jump. So
/// the suspension is the cross-check's and nothing else's, which is the point of the scenario:
/// the second source is what catches a single wrong price, and one source could not.
#[test]
fn a_post_that_disagrees_by_more_than_cross_check_suspends() {
    let Some(f) = Fixture::new() else { return };
    f.advance(100);
    f.post(ONE + 2 * BP, f.now());
    f.refresh();
    f.advance(100);
    f.publish_feed(f.at_feed_scale(ONE + 4 * BP), f.now());
    f.refresh();
    f.provide_liquidity(LIQUIDITY);
    let agreed = f.rate_state();

    f.advance(99_800);
    f.post(ONE + 13 * BP, f.now());
    let change = f.refresh();

    assert_eq!(change.before, RateStatus::Active);
    assert_eq!(
        change.after,
        RateStatus::Suspended(SuspendReason::SourcesDisagree)
    );
    let state = f.rate_state();
    assert_eq!(state.status, Status::SourcesDisagree);
    assert_eq!(
        (state.low, state.high, state.updated_at),
        (agreed.low, agreed.high, agreed.updated_at),
        "a suspension keeps the last accepted interval"
    );
    assert_eq!(
        state.last_by_source[ISSUER].map(|o| o.rate),
        Some(ONE + 2 * BP),
        "the refused observation was not stored"
    );
    assert_eq!(quote_error(&f), code::RATE_SUSPENDED_SOURCES_DISAGREE);
}

/// The feed goes down: its adapter's call fails, the pool reports the source as one without an
/// observation and carries on with the other one. The rate is still served while the lost
/// source's freshness bound runs and through the grace window that follows it, and refused the
/// second after that -- the pool does not trade on a rate only one source ever confirmed.
#[test]
fn a_dead_adapter_drops_out_and_the_rate_is_lost_after_grace() {
    let Some(f) = Fixture::new() else { return };
    f.advance(100);
    let both_at = f.now();
    f.post(ONE + 2 * BP, both_at);
    f.publish_feed(f.at_feed_scale(ONE + 2 * BP), both_at);
    f.refresh();
    f.provide_liquidity(LIQUIDITY);
    assert!(f.pool().quote_exact_in(&Direction::AssetIn, &ONE_TOKEN) > 0);

    f.fail_feed(true);
    f.advance(50_000);
    f.post(ONE + 2 * BP, f.now());
    let change = f.refresh();

    assert_eq!(
        change.after,
        RateStatus::Active,
        "a dead source is not a suspension"
    );
    assert_eq!(
        f.last_refresh_outcomes(),
        std::vec![SourceOutcome::NoObservation, SourceOutcome::Accepted],
        "the failing adapter is a source without an observation"
    );
    // The feed is dead, but its last reading is not: an observation confirms a partner for
    // as long as it is live, so the issuer's post is accepted and carries the interval's time
    // forward at the same rate. That is exactly why a lost source is not felt at once, and why
    // the moment it is felt is the end of that reading's liveness plus the grace window.
    let state = f.rate_state();
    assert_eq!(
        (state.low, state.high),
        (ONE + 2 * BP, ONE + 2 * BP),
        "the rate did not move"
    );
    assert_eq!(
        state.last_by_source[REDSTONE].map(|o| o.at),
        Some(both_at),
        "the feed published nothing new"
    );

    // The lost source's last observation was live until `both_at + max_age`; the rate is
    // served for `grace` beyond that and not a second longer.
    let last_second = both_at + 108_000 + 7_200;
    f.advance(last_second - f.now());
    assert!(f.pool().quote_exact_in(&Direction::AssetIn, &ONE_TOKEN) > 0);
    f.advance(1);
    assert_eq!(quote_error(&f), code::RATE_SOURCE_LOST);
}

/// A feed that holds no reading for the pool's asset answers `None`, the adapter passes the
/// `None` on, and the pool reports that source as one without an observation in the `refresh`
/// event -- it is not an error, not a zero, and not the other asset's price.
#[test]
fn a_none_from_the_feed_is_skipped_and_reported() {
    let Some(f) = Fixture::new() else { return };
    f.publish_feed_for_another_asset(f.at_feed_scale(ONE + 900 * BP), f.now());
    f.advance(100);
    f.post(ONE + 2 * BP, f.now());
    let change = f.refresh();

    assert_eq!(change.after, RateStatus::Active);
    assert_eq!(
        f.last_refresh_outcomes(),
        std::vec![SourceOutcome::NoObservation, SourceOutcome::Accepted]
    );
    let state = f.rate_state();
    assert!(state.last_by_source[REDSTONE].is_none());
    assert_eq!((state.low, state.high), (ONE, ONE));
}

/// The version the fixture publishes, read off the pool artefact the brand's testnet
/// deployment records.
///
/// It is asserted here because it is a number this repository acts on and does not own: every
/// deploy script reads `version()` back after its constructor and stops when it is not the
/// number it expected, so a pool artefact whose version moved silently
/// turns every deployment into a stop the operator has to diagnose on the chain. The E-007
/// amendment for the literal byte types raised it from 4 to 5, the sentinel (E-007 §3.1) from 5
/// to 6, the announced changes of rate, roles and cap from 6 to 8 (no pool is deployed on 7),
/// and this is the test
/// that says which number the bytes in `tests/fixtures/` actually carry.
///
/// The companion check -- that a deployment expects this same number -- lives with the
/// deployment's own tests, beside the deployment.
#[test]
fn the_pool_fixture_publishes_version_eight() {
    let Some(f) = Fixture::new() else { return };
    assert_eq!(
        f.pool().version(),
        8,
        "the pool Wasm in tests/fixtures/ publishes another version than a deployment reads \
         back; refresh the fixture together with the record of the deployed bytes, and move \
         the number every deployment expects with them"
    );
}

// --- The roles (E-007 §3.1 and §3.2 as amended for `VERSION` 6 and 8) --------------------------
//
// What the deploy scripts rely on, driven through the pool's Wasm: the constructor takes the three
// roles, the sentinel as `Some` or `None`, and `roles()` gives back exactly what it was given, so
// that the deploy key need hold none of them. A later change of the roles is
// announced and set a day later, all three at once, so that an announcement that left the
// sentinel out would clear it.
// The pause itself is the sentinel's reason for the role, and it is asserted once here so that
// the fixture is known to be the Wasm that has it.

#[test]
fn a_pool_created_with_a_sentinel_names_it_in_its_roles() {
    let Some(f) = Fixture::new() else { return };
    let roles = f.pool().roles();
    assert_eq!(roles.governance, f.governance);
    assert_eq!(roles.guardian, f.guardian);
    assert!(
        f.sentinel.is_some(),
        "this fixture is the one with a sentinel"
    );
    assert_eq!(roles.sentinel, f.sentinel);
}

#[test]
fn a_pool_created_without_a_sentinel_names_none() {
    let Some(f) = Fixture::without_sentinel() else {
        return;
    };
    let roles = f.pool().roles();
    assert_eq!(roles.governance, f.governance);
    assert_eq!(roles.guardian, f.guardian);
    assert_eq!(roles.sentinel, None);
}

/// The delay between `announce_roles` and `set_roles` (E-007 §3.2): `UPGRADE_DELAY`, a day.
const ROLES_DELAY: u64 = 24 * 60 * 60;

#[test]
fn naming_the_sentinel_later_is_announced_and_set_a_day_after() {
    let Some(f) = Fixture::without_sentinel() else {
        return;
    };
    let governance = Address::generate(&f.env);
    let guardian = Address::generate(&f.env);
    let sentinel = Address::generate(&f.env);

    // The later step of the testnet runbook: the pause-only key is named on a pool that was
    // deployed without one.
    f.pool().announce_roles(
        &f.governance,
        &governance,
        &guardian,
        &Some(sentinel.clone()),
    );
    assert_eq!(
        f.pool().roles().sentinel,
        None,
        "an announcement changed the roles before its day"
    );
    let code = match f.pool().try_set_roles(&f.governance) {
        Err(Ok(e)) => e.get_code(),
        other => panic!("set_roles before the delay was not refused: {other:?}"),
    };
    assert_eq!(code, code::TOO_EARLY);

    f.env
        .ledger()
        .with_mut(|l| l.timestamp = l.timestamp.saturating_add(ROLES_DELAY));
    f.pool().set_roles(&f.governance);
    let roles = f.pool().roles();
    assert_eq!(
        (roles.governance, roles.guardian, roles.sentinel),
        (governance.clone(), guardian.clone(), Some(sentinel))
    );

    // And an announcement that gives no sentinel clears it: the three are set together, always.
    f.pool()
        .announce_roles(&governance, &governance, &guardian, &None);
    f.env
        .ledger()
        .with_mut(|l| l.timestamp = l.timestamp.saturating_add(ROLES_DELAY));
    f.pool().set_roles(&governance);
    assert_eq!(f.pool().roles().sentinel, None);
}

#[test]
fn an_announcement_the_guardian_cancels_changes_no_role() {
    // The local rollout announces in both sentinel shapes and cancels each (rollout-local.sh);
    // what it relies on is that a cancelled announcement is never set.
    let Some(f) = Fixture::new() else { return };
    let before = f.pool().roles();
    f.pool().announce_roles(
        &f.governance,
        &Address::generate(&f.env),
        &Address::generate(&f.env),
        &None,
    );
    f.pool().cancel_roles(&f.guardian);
    f.env
        .ledger()
        .with_mut(|l| l.timestamp = l.timestamp.saturating_add(ROLES_DELAY));
    assert!(
        f.pool().try_set_roles(&f.governance).is_err(),
        "a cancelled announcement was set"
    );
    assert_eq!(f.pool().roles(), before);
}

#[test]
fn the_sentinel_can_pause() {
    let Some(f) = Fixture::new() else { return };
    let sentinel = f.sentinel.clone().unwrap();
    assert!(!f.pool().paused());
    f.pool().pause(&sentinel);
    assert!(
        f.pool().paused(),
        "the sentinel's pause did not pause the pool"
    );
}

#[test]
fn without_a_sentinel_no_third_key_can_pause() {
    let Some(f) = Fixture::without_sentinel() else {
        return;
    };
    let stranger = Address::generate(&f.env);
    let code = match f.pool().try_pause(&stranger) {
        Err(Ok(e)) => e.get_code(),
        other => panic!("a pause by a key that holds no role was not refused: {other:?}"),
    };
    assert_eq!(code, code::UNAUTHORISED);
    assert!(!f.pool().paused());
}

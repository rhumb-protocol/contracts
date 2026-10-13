//! Spec A-001 §4 test 2, the last line: `post` extends the instance TTL when it is below the
//! threshold.
//!
//! Why a contract that otherwise decides nothing carries this: an adapter whose instance entry
//! expires takes its price with it, and every pool of that asset loses a source at once. The
//! keeper's weekly extend-footprint-TTL operation (spec §3.4) is the first line, and it runs
//! on the same unattended machine that can be down; a post that is going to write the entry
//! anyway can extend it for almost nothing, so it does. The values are the reference values of
//! E-007 §3.1, so that an adapter and the pools reading it expire on the same schedule.

use super::harness::{Fixture, JAN_1_2026};
use super::Mode;
use crate::{INSTANCE_TTL, INSTANCE_TTL_THRESHOLD};
use soroban_sdk::testutils::Address as _;

/// A price of one, at 18 decimals.
const ONE: i128 = 1_000_000_000_000_000_000;

both!(
    a_post_below_the_threshold_extends_the_instance,
    |f: &Fixture| {
        // Spend the entry down to one ledger under the threshold.
        let spend = f.instance_ttl() - INSTANCE_TTL_THRESHOLD + 1;
        f.advance(spend);
        assert!(f.instance_ttl() < INSTANCE_TTL_THRESHOLD);
        assert_eq!(f.post(ONE, f.now() - 60), Ok(()));
        assert_eq!(f.instance_ttl(), INSTANCE_TTL);
    }
);

both!(
    a_post_above_the_threshold_leaves_the_ttl_alone,
    |f: &Fixture| {
        // Nothing to do yet, so nothing is spent on doing it.
        let before = f.instance_ttl();
        assert!(before > INSTANCE_TTL_THRESHOLD);
        assert_eq!(f.post(ONE, JAN_1_2026 - 60), Ok(()));
        assert_eq!(f.instance_ttl(), before);
    }
);

both!(
    a_refused_post_does_not_extend_the_instance,
    |f: &Fixture| {
        // A caller that cannot post cannot keep the entry alive either; otherwise anyone could
        // pay the adapter's rent with a stream of refused calls and nothing would show it.
        let spend = f.instance_ttl() - INSTANCE_TTL_THRESHOLD + 1;
        f.advance(spend);
        let before = f.instance_ttl();
        assert!(f.post(-1, f.now() - 60).is_err());
        assert_eq!(f.instance_ttl(), before);
    }
);

#[test]
fn the_threshold_is_half_the_ttl_of_the_engines_reference() {
    // Pinned so that a change to either value has to be argued for against E-007 §3.1.
    assert_eq!(INSTANCE_TTL, 30 * 24 * 60 * 60 / 5);
    assert_eq!(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL / 2);
}

both!(
    a_rotation_below_the_threshold_extends_the_instance,
    |f: &Fixture| {
        // The two role setters write the entry as a post does, and extend it on the same rule:
        // a governance that rotates a stolen keeper on an adapter nobody has posted to for a
        // while must not find the instance a few ledgers from expiry afterwards.
        let spend = f.instance_ttl() - INSTANCE_TTL_THRESHOLD + 1;
        f.advance(spend);
        assert!(f.instance_ttl() < INSTANCE_TTL_THRESHOLD);
        let new = soroban_sdk::Address::generate(&f.env);
        assert_eq!(f.set_keeper(&new), Ok(()));
        assert_eq!(f.instance_ttl(), INSTANCE_TTL);
    }
);

both!(
    a_handover_below_the_threshold_extends_the_instance,
    |f: &Fixture| {
        let spend = f.instance_ttl() - INSTANCE_TTL_THRESHOLD + 1;
        f.advance(spend);
        assert!(f.instance_ttl() < INSTANCE_TTL_THRESHOLD);
        let new = soroban_sdk::Address::generate(&f.env);
        assert_eq!(f.set_governance(&new), Ok(()));
        assert_eq!(f.instance_ttl(), INSTANCE_TTL);
    }
);

both!(
    a_rotation_above_the_threshold_leaves_the_ttl_alone,
    |f: &Fixture| {
        let before = f.instance_ttl();
        assert!(before > INSTANCE_TTL_THRESHOLD);
        let new = soroban_sdk::Address::generate(&f.env);
        assert_eq!(f.set_keeper(&new), Ok(()));
        assert_eq!(f.instance_ttl(), before);
    }
);

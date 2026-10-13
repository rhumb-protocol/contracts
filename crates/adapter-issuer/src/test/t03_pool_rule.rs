//! Spec A-001 §4 test 3, this adapter's half: what the pool accepts is the pool's rule, and
//! this adapter neither narrows it nor widens it.
//!
//! This adapter is not quite the pass-through `adapter-redstone` is: `post` refuses three
//! things. So the claim is narrower and has to be stated exactly. Of the pool's four
//! conditions, `post` overlaps with one and only one -- a price at or below zero the pool
//! refuses and `post` refuses too -- and that overlap loses the pool nothing, because a
//! refused post never becomes a reading at all. For the other three the adapter is silent:
//! a rate below `10^12` or above `10^24`, and a timestamp above `2^40`, are stored and handed
//! over, and the pool does the refusing and says so in its `refresh` event.
//!
//! The direction that would be a real fault is the other one: the adapter must never hand the
//! pool something the pool accepts when what the keeper posted was something it would not.
//! That is what "stores and reports exactly what was posted" gives, and it is asserted here on
//! both sides of every boundary the two rules share.

use super::harness::{Fixture, JAN_1_2026};
use super::Mode;
use crate::error::Error;
use proptest::prelude::*;

/// The most decimals E-003 §3.2 accepts.
const DECIMALS_MAX: u32 = 18;
/// The smallest rate E-003 §3.2 accepts, `10^12` in 18-decimal units.
const RATE_MIN: u128 = 1_000_000_000_000;
/// The largest rate E-003 §3.2 accepts, `10^24` in 18-decimal units.
const RATE_MAX: u128 = 1_000_000_000_000_000_000_000_000;
/// The largest timestamp E-003 §3.2 accepts.
const TIMESTAMP_MAX: u64 = 1 << 40;

/// E-003 §3.2's acceptance rule, restated.
fn pool_accepts(price: i128, decimals: u32, timestamp: u64) -> bool {
    if price <= 0 || decimals > DECIMALS_MAX || timestamp > TIMESTAMP_MAX {
        return false;
    }
    let Ok(price) = u128::try_from(price) else {
        return false;
    };
    let Some(scale) = 10u128.checked_pow(DECIMALS_MAX - decimals) else {
        return false;
    };
    let Some(rate) = price.checked_mul(scale) else {
        return false;
    };
    (RATE_MIN..=RATE_MAX).contains(&rate)
}

both!(
    a_price_the_pool_refuses_for_its_range_is_still_stored,
    |f: &Fixture| {
        // Under the pool's rate floor: the adapter stores it and the pool reports it as ignored,
        // which is what tells the owner that the keeper is posting nonsense. An adapter that
        // refused it here would leave the pool looking at a source that had simply gone quiet.
        let below = 999_999_999_999i128;
        assert!(!pool_accepts(below, 18, JAN_1_2026 - 60));
        assert_eq!(f.post(below, JAN_1_2026 - 60), Ok(()));
        assert_eq!(f.observe(), Some((below, 18, JAN_1_2026 - 60)));
    }
);

both!(
    a_price_above_the_pools_rate_ceiling_is_still_stored,
    |f: &Fixture| {
        let above = 1_000_000_000_000_000_000_000_001i128;
        assert!(!pool_accepts(above, 18, JAN_1_2026 - 60));
        assert_eq!(f.post(above, JAN_1_2026 - 60), Ok(()));
        assert_eq!(f.observe(), Some((above, 18, JAN_1_2026 - 60)));
    }
);

both!(the_largest_price_there_is_is_still_stored, |f: &Fixture| {
    assert!(!pool_accepts(i128::MAX, 18, JAN_1_2026 - 60));
    assert_eq!(f.post(i128::MAX, JAN_1_2026 - 60), Ok(()));
    assert_eq!(f.observe(), Some((i128::MAX, 18, JAN_1_2026 - 60)));
});

both!(
    the_only_condition_the_two_rules_share_is_the_sign,
    |f: &Fixture| {
        // The pool refuses `price <= 0` and so does `post`. Refusing it here costs the pool
        // nothing, because a refused post never becomes a reading; and it keeps a non-price out
        // of the entry, where it would sit as the newest publication and block that second.
        for price in [0i128, -1, i128::MIN] {
            assert!(!pool_accepts(price, 18, JAN_1_2026 - 60));
            assert_eq!(f.post(price, JAN_1_2026 - 60), Err(Error::BadPrice));
        }
        assert_eq!(f.observe(), None);
    }
);

#[test]
fn a_timestamp_the_pool_refuses_is_stored_when_the_ledger_allows_it() {
    // A publication time above the pool's 2^40 bound. The contract's own rule is only "not
    // later than the ledger", so with the ledger moved past that bound the post stands and
    // the pool is the one that refuses it -- and says so in its `refresh` event.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let beyond = TIMESTAMP_MAX + 1;
    f.env.ledger().with_mut(|l| l.timestamp = beyond + 1_000);
    assert!(!pool_accepts(1_000_000_000_000_000_000, 18, beyond));
    assert_eq!(f.post(1_000_000_000_000_000_000, beyond), Ok(()));
    assert_eq!(f.observe(), Some((1_000_000_000_000_000_000, 18, beyond)));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever `post` accepted comes back as the same three numbers, so the adapter can never
    /// turn a reading the pool would refuse into one it accepts.
    #[test]
    fn what_post_accepts_is_observed_unchanged(
        price in 1i128..=i128::MAX,
        timestamp in 0u64..=JAN_1_2026,
    ) {
        if let Some(f) = Fixture::new(Mode::Wasm) {
            prop_assert_eq!(f.post(price, timestamp), Ok(()));
            let observed = f.observe();
            prop_assert_eq!(observed, Some((price, 18, timestamp)));
            let (p, d, t) = observed.expect("a reading");
            // The two rules agree on this reading, whichever way round they fall.
            prop_assert_eq!(pool_accepts(p, d, t), pool_accepts(price, 18, timestamp));
        }
    }
}

use soroban_sdk::testutils::Ledger as _;

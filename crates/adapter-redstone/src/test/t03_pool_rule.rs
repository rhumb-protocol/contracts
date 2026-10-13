//! Spec A-001 §4 test 3, this adapter's half: what the pool accepts is the pool's rule, and
//! this adapter neither narrows it nor widens it.
//!
//! The rule is restated here from E-003 §3.2 -- `price > 0`, `decimals <= 18`, the rate
//! `price * 10^(18 - decimals)` inside `10^12 ..= 10^24`, and `timestamp` at most `2^40` --
//! and the test walks both sides of each of its four conditions. It is a restatement in a
//! test, not a second copy of the engine: no adapter may contain it, because an adapter that
//! applied the rule would turn a reading the pool would have reported as ignored into a
//! source that looks merely silent, and an adapter that applied a stricter one would hide a
//! live feed from the pool. The assertion in every row is the same and is the whole claim:
//! `observe` gives back what the feed gave, whether the rule accepts it or not.

use super::{Fixture, Mode};

/// The most decimals E-003 §3.2 accepts.
const DECIMALS_MAX: u32 = 18;
/// The smallest rate E-003 §3.2 accepts, `10^12` in 18-decimal units.
const RATE_MIN: u128 = 1_000_000_000_000;
/// The largest rate E-003 §3.2 accepts, `10^24` in 18-decimal units.
const RATE_MAX: u128 = 1_000_000_000_000_000_000_000_000;
/// The largest timestamp E-003 §3.2 accepts.
const TIMESTAMP_MAX: u64 = 1 << 40;

/// E-003 §3.2's acceptance rule, restated. Returns whether the pool would build an
/// observation out of this reading.
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

/// Readings on both sides of each of the rule's four conditions: the sign of the price, the
/// scale, the two ends of the rate range, and the timestamp bound.
fn readings() -> std::vec::Vec<(&'static str, i128, u32, u64)> {
    std::vec![
        (
            "one, at 18 decimals",
            1_000_000_000_000_000_000,
            18,
            1_757_000_000
        ),
        ("one, at 6 decimals", 1_000_000, 6, 1_757_000_000),
        ("zero price", 0, 18, 1_757_000_000),
        ("negative price", -1, 18, 1_757_000_000),
        ("the smallest i128", i128::MIN, 18, 1_757_000_000),
        ("the largest i128", i128::MAX, 18, 1_757_000_000),
        ("19 decimals", 1_000_000_000_000_000_000, 19, 1_757_000_000),
        ("32 decimals", 1_000_000_000_000_000_000, 32, 1_757_000_000),
        (
            "just under the rate floor",
            999_999_999_999,
            18,
            1_757_000_000
        ),
        (
            "exactly the rate floor",
            1_000_000_000_000,
            18,
            1_757_000_000
        ),
        (
            "exactly the rate ceiling",
            1_000_000_000_000_000_000_000_000,
            18,
            1_757_000_000
        ),
        (
            "just over the rate ceiling",
            1_000_000_000_000_000_000_000_001,
            18,
            1_757_000_000
        ),
        ("timestamp zero", 1_000_000_000_000_000_000, 18, 0),
        (
            "exactly the timestamp bound",
            1_000_000_000_000_000_000,
            18,
            TIMESTAMP_MAX
        ),
        (
            "just over the timestamp bound",
            1_000_000_000_000_000_000,
            18,
            TIMESTAMP_MAX + 1
        ),
        (
            "a timestamp in milliseconds",
            1_000_000_000_000_000_000,
            18,
            1_757_000_000_000
        ),
    ]
}

both!(
    the_adapter_forwards_whatever_the_rule_says,
    |f: &Fixture| {
        for (name, price, decimals, timestamp) in readings() {
            f.feed().set_decimals(&decimals);
            f.feed().set_price(&price, &timestamp);
            assert_eq!(
                f.observe(),
                Some((price, decimals, timestamp)),
                "the adapter changed the reading: {name}"
            );
        }
    }
);

#[test]
fn the_rows_walk_both_sides_of_every_condition() {
    // Without this the test above could pass on a table that never reaches a refused reading,
    // and "the adapter does not narrow the rule" would be claimed without being shown.
    let rows = readings();
    let accepted = rows
        .iter()
        .filter(|(_, p, d, t)| pool_accepts(*p, *d, *t))
        .count();
    assert!(accepted > 0, "no row the pool would accept");
    assert!(accepted < rows.len(), "no row the pool would refuse");
}

#[test]
fn a_reading_the_pool_refuses_still_reaches_it() {
    // The narrowing half, stated once on its own: a reading outside the rule is forwarded, so
    // the pool reports the source as ignored in its `refresh` event instead of as silent.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let (price, decimals, timestamp) = (-1i128, 18u32, 1_757_000_000u64);
    assert!(!pool_accepts(price, decimals, timestamp));
    f.feed().set_price(&price, &timestamp);
    assert_eq!(f.observe(), Some((price, decimals, timestamp)));
}

#[test]
fn a_reading_the_pool_accepts_is_not_altered_into_one_it_refuses() {
    // The widening half: an accepted reading arrives accepted, with the same three numbers.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let (price, decimals, timestamp) = (1_000_000_000_000_000_000i128, 18u32, 1_757_000_000u64);
    assert!(pool_accepts(price, decimals, timestamp));
    f.feed().set_price(&price, &timestamp);
    let observed = f.observe().expect("a reading");
    assert_eq!(observed, (price, decimals, timestamp));
    assert!(pool_accepts(observed.0, observed.1, observed.2));
}

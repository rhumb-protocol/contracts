//! The constructor value a deployment builds, pinned from the pool's ABI.
//!
//! [`crate::constructor_args`] builds the one argument a pool's constructor takes for each of
//! the cases below and renders it in the JSON the `stellar` CLI reads. This test holds that
//! rendering to the committed values under `fixtures/constructor-args/`, value for value and,
//! for the rate config, byte for byte. A deployment that builds the same value is compared
//! with the same rendering where the deployment lives; the two together are the whole check.
//!
//! Why it is worth a test. Two parts of the value cannot be checked by reading: the rate
//! config, which crosses the boundary as 119 opaque bytes whose layout is E-007 §3.7's and
//! whose fields are little-endian integers at fixed offsets, and the enum variant, which
//! decides which of two shapes the pool reads the rest as. A wrong byte in the first is a pool
//! that runs on a `max_age` nobody chose; a wrong variant is a deployment that fails, at best.
//! `rate_bytes` is what the tests of A-001 §4 test 4 drive through the pool's real Wasm, so the
//! committed values here are the encoding the pool is known to read.

use crate::constructor_args::{
    band_testnet_usdy, cli_json, mainnet_usdy_rate_config, no_drift_rate_config, rate_config_hex,
    stable_testnet_pyusd, testnet_usdy_rate_config, variant,
};
use soroban_sdk::Env;

/// A committed value, without the newline that ends the file.
fn committed(text: &str) -> &str {
    text.trim_end()
}

#[test]
fn the_testnet_band_pool_renders_the_committed_value() {
    let env = Env::default();
    assert_eq!(
        cli_json(&env, &band_testnet_usdy(&env, false)).unwrap(),
        committed(include_str!(
            "fixtures/constructor-args/band-testnet-usdy.json"
        )),
        "the band pool's constructor value differs from the committed one"
    );
}

#[test]
fn the_testnet_band_pool_with_a_sentinel_renders_the_committed_value() {
    // The same value with `Some` where `None` stood, for the day the pause-only key is placed.
    let env = Env::default();
    assert_eq!(
        cli_json(&env, &band_testnet_usdy(&env, true)).unwrap(),
        committed(include_str!(
            "fixtures/constructor-args/band-testnet-usdy-sentinel.json"
        )),
        "the band pool's constructor value with a sentinel differs from the committed one"
    );
}

#[test]
fn the_testnet_stable_pool_renders_the_committed_value() {
    // A stable pool takes no rate config, no sources and no genesis rate, and its cap lives
    // inside its config (E-007 §3.2).
    let env = Env::default();
    assert_eq!(
        cli_json(&env, &stable_testnet_pyusd(&env, false)).unwrap(),
        committed(include_str!(
            "fixtures/constructor-args/stable-testnet-pyusd.json"
        )),
        "the stable pool's constructor value differs from the committed one"
    );
}

#[test]
fn the_testnet_stable_pool_with_a_sentinel_renders_the_committed_value() {
    let env = Env::default();
    assert_eq!(
        cli_json(&env, &stable_testnet_pyusd(&env, true)).unwrap(),
        committed(include_str!(
            "fixtures/constructor-args/stable-testnet-pyusd-sentinel.json"
        )),
        "the stable pool's constructor value with a sentinel differs from the committed one"
    );
}

#[test]
fn the_variant_is_the_kind() {
    // The variant is what tells the pool which of the two shapes it is reading, so it is
    // asserted by name as well as inside the rendered value.
    let env = Env::default();
    assert_eq!(
        variant(&env, &band_testnet_usdy(&env, false)).unwrap(),
        "Band"
    );
    assert_eq!(
        variant(&env, &stable_testnet_pyusd(&env, false)).unwrap(),
        "Stable"
    );
}

#[test]
fn the_rate_configs_are_the_committed_bytes() {
    // The 119 bytes on their own as well as inside the value: when a whole value differs, this
    // says whether it was the rate config, the part no human reading a printed value could
    // check. The mainnet shape (two sources and a cross-check) and the shape without a drift
    // band reach the two branches of the encoder the testnet pool does not.
    let env = Env::default();
    for (config, text, what) in [
        (
            testnet_usdy_rate_config(),
            include_str!("fixtures/constructor-args/rate-config-testnet-usdy.hex"),
            "the testnet USDY pool",
        ),
        (
            mainnet_usdy_rate_config(),
            include_str!("fixtures/constructor-args/rate-config-mainnet-shaped.hex"),
            "a pool of the mainnet shape",
        ),
        (
            no_drift_rate_config(),
            include_str!("fixtures/constructor-args/rate-config-no-drift.hex"),
            "a pool without a drift band",
        ),
    ] {
        let hex = rate_config_hex(&env, &config).unwrap();
        assert_eq!(
            hex.len(),
            238,
            "a rate config is 119 bytes (E-007 §3.7); the pool refuses anything else"
        );
        assert_eq!(
            hex,
            committed(text),
            "the rate config of {what} differs from the committed bytes"
        );
    }
}

#[test]
fn the_band_value_carries_the_rate_config_bytes() {
    // The two committed files agree with each other: the band value carries exactly the bytes
    // of its rate config.
    let band = committed(include_str!(
        "fixtures/constructor-args/band-testnet-usdy.json"
    ));
    let bytes = committed(include_str!(
        "fixtures/constructor-args/rate-config-testnet-usdy.hex"
    ));
    assert!(
        band.contains(&std::format!("\"rate_config\":\"{bytes}\"")),
        "the committed band value does not carry the committed rate config"
    );
}

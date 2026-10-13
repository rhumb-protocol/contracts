//! The constructor value of a pool, built with the pool's own ABI and rendered in the JSON the
//! `stellar` CLI reads for a contract argument.
//!
//! A pool's constructor takes one argument (E-007 §3.2): an enum whose variant is the kind and
//! whose payload is that kind's arguments. A deployment writes that value as JSON on a command
//! line, and two parts of it cannot be checked by reading: the rate config, which crosses the
//! boundary as 119 opaque bytes whose layout is E-007 §3.7's, and the enum variant, which
//! decides which of two shapes the pool reads the rest as. This module states both from the
//! pool's ABI, for the cases the encoder is pinned on, so that whatever builds the value for a
//! deployment can be compared with it value for value and byte for byte.
//!
//! The rendering follows the CLI's own convention: a struct is a map from field name to value,
//! an enum is its variant name against its payload, a `BytesN` or `Bytes` is hex, a `u64` and
//! a wide integer are decimal strings, and `None` is `null`. Only the shapes a pool constructor
//! carries are rendered; anything else is an [`Unrendered`] rather than something plausible.
//!
//! The module is built for the tests of this crate and, with the feature `cli`, for a host
//! crate that compares a deployment's value with it. It is never part of a contract's Wasm.

extern crate std;

use crate::abi::{BandArgs, BandConfig, PoolArgs, StableArgs, StableConfig};
use crate::rate_bytes::{encode_config, Config, DriftBand};
use soroban_sdk::xdr::{ScVal, ScVec};
use soroban_sdk::{Address, Env, IntoVal, String as SorobanString, TryFromVal, Val, Vec};
use std::borrow::ToOwned;
use std::format;
use std::string::String;
use std::vec::Vec as StdVec;

/// The governance address of every case: an address nobody holds a key to, built from a
/// repeated byte so that it is obviously not anybody's.
pub const GOVERNANCE: &str = "GAIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCF6M";
/// The guardian address of every case, built the same way.
pub const GUARDIAN: &str = "GARCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCFRVX";
/// The sentinel of the cases that name one (E-007 §3.1), built the same way.
pub const SENTINEL: &str = "GBCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIRCEIZCA";
/// The genesis rate of every band case, one whole unit at 18 decimals, as the CLI reads it.
pub const GENESIS_RATE: &str = "1000000000000000000";

/// The genesis rate of every band case as the pool takes it.
const GENESIS_RATE_UNITS: u128 = 1_000_000_000_000_000_000;
/// The cap of every testnet case, ten thousand whole units at 18 decimals.
const CAP: u128 = 10_000_000_000_000_000_000_000;

/// The testnet ids of the test assets and the USDY issuer adapter the cases name. They are
/// testnet ids of test assets and name nothing on mainnet.
const USDY: &str = "CB3YA656OYIHU57657I5KGSBRHE5I3OZU4VFC22PYAOANFZHEWNYGAGP";
const USDC: &str = "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75";
const PYUSD: &str = "CCCRWH6Q3FNP3I2I57BDLM5AFAT7O6OF6GKQOC6SSJNDAVRZ57SPHGU2";
const USDY_ISSUER: &str = "CBI7UCH5KGSVQRO5H4SUCZUTZABCITZLRHQQZTWL2TK4RZ72TAR6IHRV";

/// A value this module does not render, with what it was.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unrendered(pub String);

/// The rate config of the testnet USDY band pool: A-001 §7.4's USDY row with the one source
/// testnet gives it and no cross-check.
pub fn testnet_usdy_rate_config() -> Config {
    Config {
        max_age: 108_000,
        max_closed_age: 108_000,
        max_step: 10,
        drift: Some(DriftBand {
            min_per_day: 0,
            max_per_day: 5,
            max_jump: 5,
        }),
        sources: 1,
        cross_check: None,
        grace: 7_200,
    }
}

/// The rate config of a band pool of the mainnet shape: two sources and a cross-check
/// (A-001 §7.4), the branch of the encoder no testnet pool reaches.
pub fn mainnet_usdy_rate_config() -> Config {
    Config {
        sources: 2,
        cross_check: Some(5),
        ..testnet_usdy_rate_config()
    }
}

/// The mainnet shape without a drift band: the cleared presence byte and the fields after it
/// on their offsets.
pub fn no_drift_rate_config() -> Config {
    Config {
        drift: None,
        ..mainnet_usdy_rate_config()
    }
}

fn address(env: &Env, id: &str) -> Address {
    Address::from_string(&SorobanString::from_str(env, id))
}

fn sentinel_of(env: &Env, sentinel: bool) -> Option<Address> {
    if sentinel {
        Some(address(env, SENTINEL))
    } else {
        None
    }
}

/// The constructor value of the testnet USDY/USDC band pool, with or without [`SENTINEL`].
pub fn band_testnet_usdy(env: &Env, sentinel: bool) -> PoolArgs {
    let mut sources = Vec::new(env);
    sources.push_back(address(env, USDY_ISSUER));
    PoolArgs::Band(BandArgs {
        governance: address(env, GOVERNANCE),
        guardian: address(env, GUARDIAN),
        sentinel: sentinel_of(env, sentinel),
        asset: address(env, USDY),
        quote: address(env, USDC),
        config: BandConfig {
            width: 10,
            fee: 5,
            bump_per_day: 5,
            bump_max: 5,
        },
        cap: CAP,
        rate_config: encode_config(env, &testnet_usdy_rate_config()),
        genesis_rate: GENESIS_RATE_UNITS,
        sources,
    })
}

/// The constructor value of the testnet PYUSD/USDC stable pool, with or without [`SENTINEL`].
pub fn stable_testnet_pyusd(env: &Env, sentinel: bool) -> PoolArgs {
    PoolArgs::Stable(StableArgs {
        governance: address(env, GOVERNANCE),
        guardian: address(env, GUARDIAN),
        sentinel: sentinel_of(env, sentinel),
        a: address(env, PYUSD),
        b: address(env, USDC),
        config: StableConfig {
            amp: 200,
            fee: 5,
            cap: CAP,
        },
    })
}

/// The value as the host holds it, in the XDR form the CLI's JSON is a rendering of.
fn sc_val(env: &Env, args: &PoolArgs) -> Result<ScVal, Unrendered> {
    let val: Val = args.clone().into_val(env);
    ScVal::try_from_val(env, &val)
        .map_err(|_| Unrendered("the value does not convert to XDR".to_owned()))
}

/// A Soroban data enum is a vector of its variant's name and its one payload.
fn enum_parts(value: &ScVal) -> Result<(String, &ScVal), Unrendered> {
    let ScVal::Vec(Some(ScVec(items))) = value else {
        return Err(Unrendered(format!("not an enum: {value:?}")));
    };
    if items.len() != 2 {
        return Err(Unrendered(format!(
            "a pool's constructor enum carries one payload: {value:?}"
        )));
    }
    let (Some(ScVal::Symbol(name)), Some(payload)) = (items.first(), items.get(1)) else {
        return Err(Unrendered(format!(
            "an enum's first item is not its variant name: {value:?}"
        )));
    };
    let name = name
        .to_utf8_string()
        .map_err(|_| Unrendered(format!("a variant name is not UTF-8: {value:?}")))?;
    Ok((name, payload))
}

/// Sixteen big-endian bytes from the two halves of a wide integer.
fn wide_bytes(hi: [u8; 8], lo: [u8; 8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (dst, src) in out.iter_mut().zip(hi.iter().chain(lo.iter())) {
        *dst = *src;
    }
    out
}

fn hex(bytes: impl Iterator<Item = u8>) -> String {
    bytes.map(|b| format!("{b:02x}")).collect()
}

/// One value in the CLI's JSON.
fn json(value: &ScVal) -> Result<String, Unrendered> {
    match value {
        ScVal::Map(Some(map)) => {
            let mut parts = StdVec::new();
            for entry in map.iter() {
                let ScVal::Symbol(name) = &entry.key else {
                    return Err(Unrendered(format!(
                        "a struct's field name is not a symbol: {:?}",
                        entry.key
                    )));
                };
                let name = name.to_utf8_string().map_err(|_| {
                    Unrendered(format!("a field name is not UTF-8: {:?}", entry.key))
                })?;
                parts.push(format!("\"{name}\":{}", json(&entry.val)?));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
        ScVal::Vec(Some(items)) => {
            let parts = items.iter().map(json).collect::<Result<StdVec<_>, _>>()?;
            Ok(format!("[{}]", parts.join(",")))
        }
        ScVal::U32(n) => Ok(format!("{n}")),
        ScVal::U64(n) => Ok(format!("\"{n}\"")),
        ScVal::U128(parts) => Ok(format!(
            "\"{}\"",
            u128::from_be_bytes(wide_bytes(parts.hi.to_be_bytes(), parts.lo.to_be_bytes()))
        )),
        ScVal::I128(parts) => Ok(format!(
            "\"{}\"",
            i128::from_be_bytes(wide_bytes(parts.hi.to_be_bytes(), parts.lo.to_be_bytes()))
        )),
        ScVal::Bytes(bytes) => Ok(format!("\"{}\"", hex(bytes.iter().copied()))),
        ScVal::Address(address) => Ok(format!("\"{address}\"")),
        // An `Option` that is `None` crosses as `Void`, which the CLI reads from JSON `null`.
        ScVal::Void => Ok("null".to_owned()),
        other => Err(Unrendered(format!(
            "a pool constructor carries no {other:?}"
        ))),
    }
}

/// The variant name of a constructor value, `Band` or `Stable`.
pub fn variant(env: &Env, args: &PoolArgs) -> Result<String, Unrendered> {
    let value = sc_val(env, args)?;
    Ok(enum_parts(&value)?.0)
}

/// A constructor value in the CLI's JSON, on one line, as a deployment prints it: the variant's
/// name against its payload.
pub fn cli_json(env: &Env, args: &PoolArgs) -> Result<String, Unrendered> {
    let value = sc_val(env, args)?;
    let (name, payload) = enum_parts(&value)?;
    Ok(format!("{{\"{name}\":{}}}", json(payload)?))
}

/// The 119 bytes of a rate config (E-007 §3.7) as lower-case hex.
pub fn rate_config_hex(env: &Env, config: &Config) -> Result<String, Unrendered> {
    let bytes = encode_config(env, config).to_array();
    Ok(hex(bytes.iter().copied()))
}

//! The encoded rate config and rate state of E-007 §3.7, written and read from this side of
//! the boundary.
//!
//! A pool takes its rate config as a fixed byte string and hands its rate state back as one,
//! so a caller has to build the first and take the second apart. The layout is the pool's
//! published one: every integer little-endian, every field in the order of E-003 §3.1, every
//! optional part a presence byte (`0` or `1`) followed by its payload, which is all zeros when
//! the part is absent, and every enum one byte. A config is [`CONFIG_LEN`] bytes and a state
//! is [`STATE_LEN`] bytes; the pool refuses anything else, so a mistake here is a refused call
//! and never a misread number.
//!
//! A config, field by field: `max_age` u64, `max_closed_age` u64, `max_step` u32, the drift
//! band (a presence byte, `min_per_day` i32, `max_per_day` i32, `max_jump` u32), the market
//! hours (a presence byte and seventy-one bytes), `sources` u8, the cross-check (a presence
//! byte and a u32) and `grace` u64. A state: `low` u128, `high` u128, `updated_at` u64,
//! `accepted_at` u64, a status byte, then four slots of a presence byte, a rate u128, a time
//! u64 and a source index.
//!
//! # Market hours are not written here
//!
//! [`Config`] has no market-hours field: no pool of spec A-001 §4 has one (USDY and USTRY take
//! `hours: None`, A-001 §7.4), so the encoder writes the cleared presence byte and its zeros.
//! The asset that does need market hours is deJTRSY, which no pool of this spec points at; the
//! deployment writes that half when a pool needs it, against the same layout.
//!
//! # Provenance
//!
//! Nothing here is ported. The layout is read in `wadrock/core` (E-007 §3.7) and written out
//! above; the encoder and the reader are this crate's own, addressing each field at a fixed
//! offset rather than walking a cursor, so that what a reader has to check is a table of
//! numbers against that list and not the behaviour of a loop.

use crate::abi::{RateConfigBytes, RateStateBytes, CONFIG_LEN, STATE_LEN};
use soroban_sdk::{BytesN, Env};

/// The drift band of E-003 §3.1: the relative change per day a rate may make, in basis
/// points, plus a jump either way.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriftBand {
    /// The least the rate may move per day.
    pub min_per_day: i32,
    /// The most it may move per day.
    pub max_per_day: i32,
    /// What it may move on top of that in one step, either way.
    pub max_jump: u32,
}

/// A rate config as E-003 §3.1 states it, minus the market hours (see the module
/// documentation). Durations are seconds, bounds basis points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    /// How old a rate may be and still be read.
    pub max_age: u64,
    /// The same while the market of the asset is closed.
    pub max_closed_age: u64,
    /// The most one observation may move the interval.
    pub max_step: u32,
    /// The drift band, when the asset has one.
    pub drift: Option<DriftBand>,
    /// How many sources the pool reads.
    pub sources: u8,
    /// How far two sources may differ before the rate suspends.
    pub cross_check: Option<u32>,
    /// How long a rate is still served after a second source was lost.
    pub grace: u64,
}

/// Where each field of a config sits, in bytes from the start. Written out as offsets rather
/// than walked with a cursor, because the layout is fixed and an offset is a thing a reader can
/// check against the list in the module documentation without following any code.
mod config_at {
    pub const MAX_AGE: usize = 0;
    pub const MAX_CLOSED_AGE: usize = 8;
    pub const MAX_STEP: usize = 16;
    pub const DRIFT_PRESENT: usize = 20;
    pub const MIN_PER_DAY: usize = 21;
    pub const MAX_PER_DAY: usize = 25;
    pub const MAX_JUMP: usize = 29;
    pub const HOURS_PRESENT: usize = 33;
    pub const SOURCES: usize = 105;
    pub const CROSS_PRESENT: usize = 106;
    pub const CROSS_CHECK: usize = 107;
    pub const GRACE: usize = 111;
}

/// Where each field of a state sits. The four observation slots follow the header, twenty-six
/// bytes apart: a presence byte, a rate, a time and a source index.
mod state_at {
    pub const LOW: usize = 0;
    pub const HIGH: usize = 16;
    pub const UPDATED_AT: usize = 32;
    pub const ACCEPTED_AT: usize = 40;
    pub const STATUS: usize = 48;
    pub const FIRST_SLOT: usize = 49;
    pub const SLOT: usize = 26;
    pub const SLOT_RATE: usize = 1;
    pub const SLOT_AT: usize = 17;
    pub const SLOT_SOURCE: usize = 25;
}

/// Copies `bytes` into `buffer` at `at`. A write that would not fit is dropped, which the fixed
/// offsets above make impossible and which the length assertion at the end of the encoder
/// holds anyway.
fn put(buffer: &mut [u8], at: usize, bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        if let Some(slot) = at.checked_add(index).and_then(|i| buffer.get_mut(i)) {
            *slot = *byte;
        }
    }
}

/// The `N` bytes of `buffer` at `at`, or zeros when they are not there. The callers read inside
/// a byte string whose length the contract fixes, so the fallback cannot be reached.
fn take<const N: usize>(buffer: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    for (index, slot) in out.iter_mut().enumerate() {
        if let Some(byte) = at.checked_add(index).and_then(|i| buffer.get(i)) {
            *slot = *byte;
        }
    }
    out
}

/// Encodes a rate config for the pool's constructor and for `set_rate`.
///
/// Market hours are left absent: a cleared presence byte and the zeros behind it (see the
/// module documentation).
pub fn encode_config(env: &Env, config: &Config) -> RateConfigBytes {
    let mut out = [0u8; CONFIG_LEN];
    put(&mut out, config_at::MAX_AGE, &config.max_age.to_le_bytes());
    put(
        &mut out,
        config_at::MAX_CLOSED_AGE,
        &config.max_closed_age.to_le_bytes(),
    );
    put(
        &mut out,
        config_at::MAX_STEP,
        &config.max_step.to_le_bytes(),
    );
    if let Some(drift) = config.drift {
        put(&mut out, config_at::DRIFT_PRESENT, &[1]);
        put(
            &mut out,
            config_at::MIN_PER_DAY,
            &drift.min_per_day.to_le_bytes(),
        );
        put(
            &mut out,
            config_at::MAX_PER_DAY,
            &drift.max_per_day.to_le_bytes(),
        );
        put(&mut out, config_at::MAX_JUMP, &drift.max_jump.to_le_bytes());
    }
    put(&mut out, config_at::HOURS_PRESENT, &[0]);
    put(&mut out, config_at::SOURCES, &[config.sources]);
    if let Some(bps) = config.cross_check {
        put(&mut out, config_at::CROSS_PRESENT, &[1]);
        put(&mut out, config_at::CROSS_CHECK, &bps.to_le_bytes());
    }
    put(&mut out, config_at::GRACE, &config.grace.to_le_bytes());
    BytesN::from_array(env, &out)
}

/// The rate's status as the state carries it: one byte, `0` active and `1 ..= 4` suspended
/// with the reason in the order of E-003's `Reason`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// The rate may be read.
    Active,
    /// One observation moved further than `max_step`.
    StepTooLarge,
    /// An observation left the drift band.
    OutsideDriftBand,
    /// Two live sources differ by more than `cross_check`.
    SourcesDisagree,
    /// The guardian suspended the rate.
    Manual,
    /// A byte the layout does not define. Kept rather than refused so that a test says what it
    /// read instead of failing somewhere else.
    Unknown(u8),
}

/// One source's stored observation (E-003 §3.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    /// The rate the source published, at 18 decimals.
    pub rate: u128,
    /// Its publication time, in seconds.
    pub at: u64,
    /// Which configured source it came from.
    pub source: u8,
}

/// A rate state as the pool hands it back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct State {
    /// The low edge of the accepted interval.
    pub low: u128,
    /// Its high edge.
    pub high: u128,
    /// The publication time the interval carries.
    pub updated_at: u64,
    /// The chain time at which it was accepted.
    pub accepted_at: u64,
    /// Whether the rate is active, and why not when it is not.
    pub status: Status,
    /// The newest observation of each configured source.
    pub last_by_source: [Option<Observation>; 4],
}

/// Reads a state the pool returned. Total: every field is fixed-size at a fixed offset, and the
/// length is the contract's own.
pub fn decode_state(bytes: &RateStateBytes) -> State {
    let raw = bytes.to_array();
    debug_assert_eq!(raw.len(), STATE_LEN);
    let mut last_by_source = [None; 4];
    for (index, slot) in last_by_source.iter_mut().enumerate() {
        let at = state_at::FIRST_SLOT.saturating_add(index.saturating_mul(state_at::SLOT));
        let present = take::<1>(&raw, at);
        *slot = (present == [1]).then(|| Observation {
            rate: u128::from_le_bytes(take(&raw, at.saturating_add(state_at::SLOT_RATE))),
            at: u64::from_le_bytes(take(&raw, at.saturating_add(state_at::SLOT_AT))),
            source: u8::from_le_bytes(take(&raw, at.saturating_add(state_at::SLOT_SOURCE))),
        });
    }
    State {
        low: u128::from_le_bytes(take(&raw, state_at::LOW)),
        high: u128::from_le_bytes(take(&raw, state_at::HIGH)),
        updated_at: u64::from_le_bytes(take(&raw, state_at::UPDATED_AT)),
        accepted_at: u64::from_le_bytes(take(&raw, state_at::ACCEPTED_AT)),
        status: match u8::from_le_bytes(take(&raw, state_at::STATUS)) {
            0 => Status::Active,
            1 => Status::StepTooLarge,
            2 => Status::OutsideDriftBand,
            3 => Status::SourcesDisagree,
            4 => Status::Manual,
            other => Status::Unknown(other),
        },
        last_by_source,
    }
}

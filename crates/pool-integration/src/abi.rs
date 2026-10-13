//! The pool's interface as a caller must state it (E-007 §3.2, §3.7).
//!
//! Every type here crosses the contract boundary, so its shape is part of the call: a struct
//! is a map from field name to value and a data enum is its variant's name with its payload,
//! which makes a renamed field or a reordered variant a different call rather than a compile
//! error. They are written from the pool's published interface and are proved by use -- the
//! tests of this crate reach the pool's Wasm with them.
//!
//! The client's methods carry no `Result`: a Soroban call that succeeds returns the payload
//! and a call that fails returns an error code, and declaring the codes here would mean
//! restating the engine's error enum in this repository to assert three of its numbers. The
//! generated `try_` methods hand back `soroban_sdk::Error` instead, and the tests name the
//! codes of E-007 §3.5 they expect one by one, in [`code`].

// The SDK's `contracttype` and `contractclient` macros generate items without doc comments,
// which the workspace's `missing_docs = "deny"` would reject; the items this file writes by
// hand are documented above each of them.
#![allow(missing_docs)]

use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

/// Bytes of an encoded rate config (E-007 §3.7).
pub const CONFIG_LEN: usize = 119;

/// Bytes of an encoded rate state (E-007 §3.7).
pub const STATE_LEN: usize = 153;

/// A rate config as the pool carries it.
pub type RateConfigBytes = BytesN<CONFIG_LEN>;

/// A rate state as the pool carries it.
pub type RateStateBytes = BytesN<STATE_LEN>;

/// The error codes of E-007 §3.5 these tests name. A code is asserted by number rather than
/// by variant so that this repository states exactly which of the engine's numbers it depends
/// on, and so that a code that moved shows up as the failure it would be on a chain.
pub mod code {
    /// `Unauthorised`: the caller holds no role that may make the call (E-007 §3.5).
    pub const UNAUTHORISED: u32 = 4;
    /// `TooEarly`: an announced change executed before its delay has passed (E-007 §3.2,
    /// `VERSION` 8).
    pub const TOO_EARLY: u32 = 17;
    /// `rate::RateError::SourceLost`: fewer than two sources live and the grace window spent.
    pub const RATE_SOURCE_LOST: u32 = 144;
    /// `rate::RateError::Suspended(SourcesDisagree)`.
    pub const RATE_SUSPENDED_SOURCES_DISAGREE: u32 = 147;
}

/// `band::PoolConfig` as the band pool's constructor takes it (E-007 §3.2): the half-width of
/// the band and the fee in basis points, and the bump the pool may add per day and at most.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BandConfig {
    pub width: u32,
    pub fee: u32,
    pub bump_per_day: u32,
    pub bump_max: u32,
}

/// The three roles of a pool (E-007 §3.1, `VERSION` 6): governance, the guardian, and the
/// sentinel, the pause-only key, which a pool created without one does not have.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Roles {
    pub governance: Address,
    pub guardian: Address,
    pub sentinel: Option<Address>,
}

/// The constructor's arguments for a band pool (E-007 §3.2). `sentinel` follows `guardian`
/// since `VERSION` 6; a struct crosses the boundary as a map keyed by field name, so the
/// position written here is for the reader and the name is what the pool reads.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BandArgs {
    pub governance: Address,
    pub guardian: Address,
    pub sentinel: Option<Address>,
    pub asset: Address,
    pub quote: Address,
    pub config: BandConfig,
    pub cap: u128,
    pub rate_config: RateConfigBytes,
    pub genesis_rate: u128,
    pub sources: Vec<Address>,
}

/// `stable_pool::PoolConfig` as the stable pool's constructor takes it (E-007 §3.2): the
/// amplification, the fee in basis points, and the cap, which a stable pool carries inside its
/// config rather than beside it as a band pool does.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StableConfig {
    pub amp: u32,
    pub fee: u32,
    pub cap: u128,
}

/// The constructor's arguments for a stable pool (E-007 §3.2). No rate config, no sources and
/// no genesis rate: a stable pool reads no rate and has no adapters. Its two tokens are `a` and
/// `b` in the order fixed at creation, not `asset` and `quote`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableArgs {
    pub governance: Address,
    pub guardian: Address,
    pub sentinel: Option<Address>,
    pub a: Address,
    pub b: Address,
    pub config: StableConfig,
}

/// What the pool's constructor takes: the kind as the variant and that kind's arguments as the
/// payload, so that kind and arguments cannot disagree (E-007 §3.2).
///
/// The band variant is the one A-001 test 4 drives through the pool's Wasm. The stable variant
/// was added for the deployment of the stable testnet pool, whose constructor value is built
/// from this type; the variant order is the pool's, because the position of a variant is part
/// of how it encodes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolArgs {
    Band(BandArgs),
    Stable(StableArgs),
}

/// Which token the trader gives on a band pool (E-007 §3.2). The two stable directions are
/// part of the same enum on the pool's side and keep their numbers here, so that `AssetIn`
/// and `QuoteIn` encode as the pool reads them.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    AssetIn = 0,
    QuoteIn = 1,
    AtoB = 2,
    BtoA = 3,
}

/// Why a rate is suspended (E-003 §3.1).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuspendReason {
    StepTooLarge = 0,
    OutsideDriftBand = 1,
    SourcesDisagree = 2,
    Manual = 3,
}

/// The rate's status (E-003 §3.1).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RateStatus {
    Active,
    Suspended(SuspendReason),
}

/// What `refresh` returns: the status before and after the call.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateStatusChange {
    pub before: RateStatus,
    pub after: RateStatus,
}

/// One source's outcome in the `refresh` event (E-007 §3.5): applied, not newer, refused with
/// the engine's code, or no observation at all -- which is what a `None` from a feed and a
/// failing adapter both become.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceOutcome {
    Accepted,
    NotNewer,
    Ignored(u32),
    NoObservation,
}

/// The part of the pool's interface these tests call.
#[soroban_sdk::contractclient(name = "PoolClient")]
#[allow(dead_code)]
pub trait PoolInterface {
    /// The pool's own version number, which every deployment reads back after a constructor
    /// and compares with the number it expects.
    fn version(env: Env) -> u32;
    /// The three roles, which the deploy script reads back after the constructor.
    fn roles(env: Env) -> Roles;
    /// A change of the roles, all three at once, announced by governance (E-007 §3.2, `VERSION`
    /// 8). A `None` sentinel announces none.
    fn announce_roles(
        env: Env,
        caller: Address,
        governance: Address,
        guardian: Address,
        sentinel: Option<Address>,
    );
    /// The announced roles, set once the delay has passed, by the governance that stands then.
    fn set_roles(env: Env, caller: Address);
    /// The announcement withdrawn, by governance or the guardian.
    fn cancel_roles(env: Env, caller: Address);
    /// By the guardian or the sentinel (E-007 §3.2, `VERSION` 6).
    fn pause(env: Env, caller: Address);
    fn paused(env: Env) -> bool;
    fn refresh(env: Env, caller: Address) -> RateStatusChange;
    fn deposit(
        env: Env,
        from: Address,
        first_max: i128,
        second_max: i128,
        min_shares: i128,
    ) -> i128;
    fn quote_exact_in(env: Env, direction: Direction, amount_in: i128) -> i128;
    fn rate_state(env: Env) -> RateStateBytes;
    fn rate_config(env: Env) -> RateConfigBytes;
}

//! One test's world: two tokens, a mock SEP-40 feed, the two adapters on it, and a band pool
//! of the engine reading both.

use crate::abi::{BandArgs, BandConfig, PoolArgs, PoolClient, RateStatusChange, SourceOutcome};
use crate::rate_bytes::{decode_state, encode_config, Config, DriftBand, State};
use adapter_redstone::sep40::Asset;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::xdr::{ContractEventBody, ScSymbol, ScVal};
use soroban_sdk::{Address, Env, IntoVal, Map, Symbol, TryFromVal, Val, Vec};
use std::cell::RefCell;
use std::path::PathBuf;

/// A ledger timestamp the tests date everything from: 1 January 2026, 00:00:00 UTC.
pub const JAN_1_2026: u64 = 1_767_225_600;

/// One in the engine's normalised scale: 18 decimals, the scale every rate is in.
pub const ONE: u128 = 1_000_000_000_000_000_000;

/// One basis point of that scale, so that a test states a price as "four basis points above
/// one" and not as a nineteen-digit number.
pub const BP: u128 = ONE / 10_000;

/// The decimals RedStone's SEP-40 feed reports (A-001 §7.2, read on chain 2026-09-12). The
/// feed is at a coarser scale than the issuer adapter's 18, which is exactly why the mock
/// answers at this one: the pool scales both, and a test that used 18 everywhere would never
/// exercise that.
pub const FEED_DECIMALS: u32 = 8;

/// The source order of the pool: the SEP-40 adapter is source 0 and the issuer adapter is
/// source 1, which is the order `sources` is written in and the order the outcomes of a
/// `refresh` event come back in.
pub const REDSTONE: usize = 0;
/// See [`REDSTONE`].
pub const ISSUER: usize = 1;

/// The USDY rate config of A-001 §7.4: 30 hours of `max_age`, a two-hour grace, ten basis
/// points of step, the drift band of `0 .. 5` basis points a day with a five-point jump, two
/// sources and a five-point cross-check. No market hours: USDY accrues every calendar day.
pub fn usdy_rate_config() -> Config {
    Config {
        max_age: 108_000,
        max_closed_age: 108_000,
        max_step: 10,
        drift: Some(DriftBand {
            min_per_day: 0,
            max_per_day: 5,
            max_jump: 5,
        }),
        sources: 2,
        cross_check: Some(5),
        grace: 7_200,
    }
}

/// The reference `PoolConfig` of A-001 §7.4 for a band pool: a band ten basis points wide, a
/// five-point fee, and the bump of five points a day to five at most.
pub fn band_config() -> BandConfig {
    BandConfig {
        width: 10,
        fee: 5,
        bump_per_day: 5,
        bump_max: 5,
    }
}

/// The cap of A-001 §7.4 at the start of the beta: 10 000 tokens, normalised.
pub const CAP: u128 = 10_000_000_000_000_000_000_000;

/// A test environment that does not write a `test_snapshots/` file when it is dropped.
fn test_env() -> Env {
    Env::new_with_config(soroban_sdk::testutils::EnvTestConfig {
        capture_snapshot_at_drop: false,
    })
}

/// The engine's pool Wasm, as core's CI built it. Required: see `tests/fixtures/README.md`.
fn pool_wasm() -> std::vec::Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pool_soroban.wasm");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "{} is missing ({e}); it is the artefact of core's CI and is placed by hand, see tests/fixtures/README.md",
            path.display()
        )
    })
}

/// A Wasm this workspace builds, or `None` when it has not been built.
fn built_wasm(name: &str) -> Option<std::vec::Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32v1-none/release")
        .join(name);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            if std::env::var("ADAPTER_WASM_REQUIRED").is_ok() {
                panic!(
                    "{} is missing; build it with: cargo build --workspace --release --target wasm32v1-none",
                    path.display()
                );
            }
            std::eprintln!(
                "notice: {} is missing, skipping; build it with: cargo build --workspace --release --target wasm32v1-none",
                path.display()
            );
            None
        }
    }
}

/// A band pool with its two adapters, the feed behind one of them, and the two tokens.
///
/// The adapters' own addresses are not kept beyond the pool's `sources`: a test here never
/// calls an adapter, it calls the pool. The roles are kept, so that the tests of the pool's
/// third role (E-007 §3.1, §3.2) can compare `roles()` with what the constructor was given; every
/// authorisation is still mocked in this environment, and the tests that hold the adapters to
/// their two keys are the adapters' own (spec A-001 §4 test 2).
pub struct Fixture {
    pub env: Env,
    pub pool: Address,
    pub governance: Address,
    pub guardian: Address,
    /// The pool's sentinel, `None` for a pool created without one.
    pub sentinel: Option<Address>,
    pub feed: Address,
    pub issuer: Address,
    pub asset: Address,
    pub quote: Address,
    pub caller: Address,
    /// The outcomes of the last `refresh`, kept because the test environment keeps only the
    /// events of the last top-level call.
    outcomes: RefCell<std::vec::Vec<SourceOutcome>>,
}

impl Fixture {
    /// The world of the scenarios: a pool at a genesis rate of one, two sources, no reading
    /// from either yet, and a sentinel. `None` when a Wasm of this workspace has not been
    /// built.
    pub fn new() -> Option<Fixture> {
        Self::build(true)
    }

    /// The same world with a pool created without a sentinel, which is how every testnet pool
    /// is deployed until the pause-only key exists.
    pub fn without_sentinel() -> Option<Fixture> {
        Self::build(false)
    }

    fn build(with_sentinel: bool) -> Option<Fixture> {
        let feed_wasm = built_wasm("sep40_mock.wasm")?;
        let redstone_wasm = built_wasm("adapter_redstone.wasm")?;
        let issuer_wasm = built_wasm("adapter_issuer.wasm")?;
        let env = test_env();
        env.mock_all_auths();
        env.ledger().with_mut(|l| {
            l.timestamp = JAN_1_2026;
            l.sequence_number = 1_000;
            l.min_persistent_entry_ttl = 4_096;
            l.max_entry_ttl = 6_312_000;
        });

        let governance = Address::generate(&env);
        let guardian = Address::generate(&env);
        let sentinel = with_sentinel.then(|| Address::generate(&env));
        let keeper = Address::generate(&env);
        let caller = Address::generate(&env);
        let admin = Address::generate(&env);

        let asset = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let quote = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();

        // The feed answers at RedStone's scale, and only for the asset the adapter asks for.
        let feed = env.register(feed_wasm.as_slice(), ());
        env.invoke_contract::<()>(
            &feed,
            &Symbol::new(&env, "set_decimals"),
            Vec::from_array(&env, [FEED_DECIMALS.into_val(&env)]),
        );

        let redstone = env.register(
            redstone_wasm.as_slice(),
            (feed.clone(), Asset::Stellar(asset.clone())),
        );
        let issuer = env.register(
            issuer_wasm.as_slice(),
            (governance.clone(), keeper.clone(), 18u32),
        );

        let sources = Vec::from_array(&env, [redstone.clone(), issuer.clone()]);
        let args = PoolArgs::Band(BandArgs {
            governance: governance.clone(),
            guardian: guardian.clone(),
            sentinel: sentinel.clone(),
            asset: asset.clone(),
            quote: quote.clone(),
            config: band_config(),
            cap: CAP,
            rate_config: encode_config(&env, &usdy_rate_config()),
            genesis_rate: ONE,
            sources,
        });
        let pool = env.register(pool_wasm().as_slice(), (args,));

        Some(Fixture {
            env,
            pool,
            governance,
            guardian,
            sentinel,
            feed,
            issuer,
            asset,
            quote,
            caller,
            outcomes: RefCell::new(std::vec::Vec::new()),
        })
    }

    /// The pool as a caller sees it.
    pub fn pool(&self) -> PoolClient<'_> {
        PoolClient::new(&self.env, &self.pool)
    }

    /// Mints `units` of both tokens to an account and deposits them, so that the pool has
    /// something to quote on. Token units, at the tokens' own seven decimals.
    pub fn provide_liquidity(&self, units: i128) {
        let lp = Address::generate(&self.env);
        for token in [&self.asset, &self.quote] {
            soroban_sdk::token::StellarAssetClient::new(&self.env, token).mint(&lp, &units);
        }
        self.pool().deposit(&lp, &units, &units, &0);
    }

    /// What the feed answers for the pool's asset from now on, at [`FEED_DECIMALS`].
    pub fn publish_feed(&self, price: i128, timestamp: u64) {
        self.env.invoke_contract::<()>(
            &self.feed,
            &Symbol::new(&self.env, "set_price_for"),
            Vec::from_array(
                &self.env,
                [
                    Asset::Stellar(self.asset.clone()).into_val(&self.env),
                    price.into_val(&self.env),
                    timestamp.into_val(&self.env),
                ],
            ),
        );
    }

    /// What the feed answers for another asset, so that a test can put a reading on the feed
    /// that is not the one the adapter asks for.
    pub fn publish_feed_for_another_asset(&self, price: i128, timestamp: u64) {
        self.env.invoke_contract::<()>(
            &self.feed,
            &Symbol::new(&self.env, "set_price_for"),
            Vec::from_array(
                &self.env,
                [
                    Asset::Other(Symbol::new(&self.env, "OTHER")).into_val(&self.env),
                    price.into_val(&self.env),
                    timestamp.into_val(&self.env),
                ],
            ),
        );
    }

    /// Makes every call to the feed fail from now on, as an oracle that is down does.
    pub fn fail_feed(&self, failing: bool) {
        self.env.invoke_contract::<()>(
            &self.feed,
            &Symbol::new(&self.env, "set_failing"),
            Vec::from_array(&self.env, [failing.into_val(&self.env)]),
        );
    }

    /// A rate at 18 decimals as the feed's own price at [`FEED_DECIMALS`]: the same number at
    /// a coarser scale, computed by division so that no test writes a scaled constant by hand.
    /// Panics when the rate is not exact at the feed's scale, which every rate a test states
    /// is.
    pub fn at_feed_scale(&self, rate: u128) -> i128 {
        let divisor = 10u128.pow(18 - FEED_DECIMALS);
        assert_eq!(
            rate % divisor,
            0,
            "the rate is not exact at the feed's scale"
        );
        i128::try_from(rate / divisor).unwrap()
    }

    /// The keeper posts a price to the issuer adapter, at 18 decimals.
    pub fn post(&self, price: u128, timestamp: u64) {
        self.env.invoke_contract::<()>(
            &self.issuer,
            &Symbol::new(&self.env, "post"),
            Vec::from_array(
                &self.env,
                [
                    i128::try_from(price).unwrap().into_val(&self.env),
                    timestamp.into_val(&self.env),
                ],
            ),
        );
    }

    /// `refresh`, as the keeper calls it after every post (spec §3.4). The outcomes of the
    /// call's `refresh` event are kept for [`Fixture::last_refresh_outcomes`]: the test
    /// environment holds the events of the last top-level call only, so a reading taken after
    /// the next view call would find nothing.
    pub fn refresh(&self) -> RateStatusChange {
        let change = self.pool().refresh(&self.caller);
        *self.outcomes.borrow_mut() = self.refresh_outcomes_of_last_call();
        change
    }

    /// The pool's stored rate state, read back and taken apart.
    pub fn rate_state(&self) -> State {
        decode_state(&self.pool().rate_state())
    }

    /// The outcomes of the last [`Fixture::refresh`], one per source in source order.
    pub fn last_refresh_outcomes(&self) -> std::vec::Vec<SourceOutcome> {
        self.outcomes.borrow().clone()
    }

    /// The outcomes in the `refresh` event of the call that just ran.
    ///
    /// The event is read from its XDR rather than compared against a rebuilt one: the event
    /// type is the pool's, this repository does not have it, and what a caller of a pool can
    /// actually see is the topics and the data map the chain carries.
    fn refresh_outcomes_of_last_call(&self) -> std::vec::Vec<SourceOutcome> {
        let events = self.env.events().all().filter_by_contract(&self.pool);
        let wanted = ScVal::Symbol(ScSymbol(
            "refresh".try_into().expect("the topic is a valid symbol"),
        ));
        let last = events
            .events()
            .iter()
            .filter_map(|event| {
                let ContractEventBody::V0(body) = &event.body;
                (body.topics.first() == Some(&wanted)).then(|| body.data.clone())
            })
            .next_back()
            .expect("the pool emitted no refresh event");
        let data: Val =
            Val::try_from_val(&self.env, &last).expect("the event's data did not decode");
        let data: Map<Symbol, Val> =
            Map::try_from_val(&self.env, &data).expect("the refresh event's data is not a map");
        let outcomes: Vec<SourceOutcome> = data
            .get(Symbol::new(&self.env, "outcomes"))
            .expect("the refresh event carries no outcomes")
            .into_val(&self.env);
        outcomes.iter().collect()
    }

    /// Moves the ledger forward by `seconds`, five seconds to the ledger.
    pub fn advance(&self, seconds: u64) {
        self.env.ledger().with_mut(|l| {
            l.timestamp += seconds;
            l.sequence_number += u32::try_from(seconds / 5).unwrap();
        });
    }

    /// The ledger's time.
    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }
}

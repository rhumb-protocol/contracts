//! What the reference tests of this crate share: the Wasm files, the mock SEP-40 feed, and the
//! two ways of driving the adapter.
//!
//! The Wasm files come from the workspace target directory, which `cargo build --workspace
//! --release --target wasm32v1-none` fills. A missing file makes [`Fixture::new`] return
//! `None` and the test pass with a notice, unless `ADAPTER_WASM_REQUIRED` is set; CI sets it
//! after the Wasm build, so a green CI cannot mean that the tests found nothing to run.

use super::{AdapterClient, Mode};
use crate::contract::RedstoneAdapter;
use crate::sep40::{Asset, PriceData};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::EnvTestConfig;
use soroban_sdk::{Address, Env};
use std::path::PathBuf;

/// A test environment that does not write a `test_snapshots/` file when it is dropped.
///
/// The SDK captures a snapshot of every `Env` at drop by default, which would put a generated
/// JSON file per test into the tree -- hundreds of them for a property test -- and none of it
/// is source. The tests here assert on values directly, so the capture is turned off and the
/// working tree stays what `git ls-files` says it is.
pub fn test_env() -> Env {
    Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    })
}

/// The mock feed's test-only setters, as a client the fixture can call.
#[soroban_sdk::contractclient(name = "MockClient")]
#[allow(dead_code)]
pub trait MockInterface {
    fn set_decimals(env: Env, decimals: u32);
    fn set_price(env: Env, price: i128, timestamp: u64);
    fn set_price_for(env: Env, asset: Asset, price: i128, timestamp: u64);
    fn clear_price(env: Env);
    fn set_failing(env: Env, failing: bool);
    fn decimals(env: Env) -> u32;
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;
}

/// Reads one Wasm from the workspace target directory, or `None` when it is not there.
fn wasm(name: &str) -> Option<std::vec::Vec<u8>> {
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

/// The mock feed's Wasm, or `None` when it has not been built.
pub fn mock_wasm_for_tests() -> Option<std::vec::Vec<u8>> {
    wasm("sep40_mock.wasm")
}

/// The adapter's own Wasm, or `None` when it has not been built.
pub fn adapter_wasm_for_tests() -> Option<std::vec::Vec<u8>> {
    wasm("adapter_redstone.wasm")
}

/// One test's world: a mock feed, an adapter pointed at it, and the mode it is driven in.
pub struct Fixture {
    pub env: Env,
    pub mode: Mode,
    /// The mock SEP-40 feed the adapter reads.
    pub oracle: Address,
    /// The adapter instance in `Wasm` mode; a shell instance the entry points run inside in
    /// `Native` mode.
    pub adapter: Address,
    /// The asset the adapter asks the feed for.
    pub asset: Asset,
}

impl Fixture {
    /// A fixture whose feed reports 18 decimals, the scale every source of Rhumb posts at.
    pub fn new(mode: Mode) -> Option<Fixture> {
        Fixture::with_feed_decimals(mode, 18)
    }

    /// A fixture whose feed reports `decimals` at the moment the adapter is constructed.
    /// Panics when the constructor refuses, which is what the constructor tests assert.
    pub fn with_feed_decimals(mode: Mode, decimals: u32) -> Option<Fixture> {
        let mock_wasm = wasm("sep40_mock.wasm")?;
        let adapter_wasm = wasm("adapter_redstone.wasm")?;
        let env = test_env();

        let oracle = env.register(mock_wasm.as_slice(), ());
        MockClient::new(&env, &oracle).set_decimals(&decimals);

        // A token contract id is what every pool of Rhumb names as its feed's asset. A test
        // that needs a second asset generates it in this fixture's own environment: an
        // address from another `Env` is not a value this one can resolve.
        let asset = Asset::Stellar(Address::generate(&env));

        let adapter = match mode {
            Mode::Wasm => env.register(adapter_wasm.as_slice(), (oracle.clone(), asset.clone())),
            Mode::Native => {
                // A shell instance: registered from the mock's Wasm, which is never invoked in
                // this mode, so that `as_contract` has storage of its own to write into.
                let shell = env.register(mock_wasm.as_slice(), ());
                env.as_contract(&shell, || {
                    RedstoneAdapter::__constructor(env.clone(), oracle.clone(), asset.clone())
                        .unwrap_or_else(|e| panic!("constructor refused: {e:?}"));
                });
                shell
            }
        };

        Some(Fixture {
            env,
            mode,
            oracle,
            adapter,
            asset,
        })
    }

    /// The mock feed's setters.
    pub fn feed(&self) -> MockClient<'_> {
        MockClient::new(&self.env, &self.oracle)
    }

    /// `observe()` in the fixture's mode.
    pub fn observe(&self) -> Option<(i128, u32, u64)> {
        match self.mode {
            Mode::Wasm => AdapterClient::new(&self.env, &self.adapter).observe(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || RedstoneAdapter::observe(self.env.clone())),
        }
    }

    /// `oracle()` in the fixture's mode.
    pub fn oracle_view(&self) -> Option<Address> {
        match self.mode {
            Mode::Wasm => AdapterClient::new(&self.env, &self.adapter).oracle(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || RedstoneAdapter::oracle(self.env.clone())),
        }
    }

    /// `asset()` in the fixture's mode.
    pub fn asset_view(&self) -> Option<Asset> {
        match self.mode {
            Mode::Wasm => AdapterClient::new(&self.env, &self.adapter).asset(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || RedstoneAdapter::asset(self.env.clone())),
        }
    }

    /// `version()` in the fixture's mode.
    pub fn version(&self) -> u32 {
        match self.mode {
            Mode::Wasm => AdapterClient::new(&self.env, &self.adapter).version(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || RedstoneAdapter::version(self.env.clone())),
        }
    }
}

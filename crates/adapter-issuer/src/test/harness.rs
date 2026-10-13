//! What the reference tests of this crate share: the Wasm, the two roles, and the two ways of
//! driving the adapter.

use super::{IssuerClient, Mode};
use crate::contract::IssuerAdapter;
use crate::error::Error;
use crate::types::Posting;
use soroban_sdk::testutils::{
    Address as _, Deployer as _, EnvTestConfig, Events as _, Ledger as _,
};
use soroban_sdk::{Address, Env, Event};
use std::path::PathBuf;

/// A ledger timestamp the tests date everything from: 1 January 2026, 00:00:00 UTC. Every
/// publication time in the tests sits near it, so that "newer" and "from the future" are
/// about the contract's rule and not about the test environment's defaults.
pub const JAN_1_2026: u64 = 1_767_225_600;

/// A test environment that does not write a `test_snapshots/` file when it is dropped.
///
/// The SDK captures a snapshot of every `Env` at drop by default, which would put a generated
/// JSON file per test into the tree and none of it is source.
pub fn test_env() -> Env {
    Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    })
}

/// The adapter's Wasm, or `None` when it has not been built.
pub fn adapter_wasm_for_tests() -> Option<std::vec::Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32v1-none/release/adapter_issuer.wasm");
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

/// The two layers of `Result` a generated `try_` call returns, flattened to the one the
/// contract's own signature has: the outer failure carries the contract's error, and the two
/// remaining cases mean the call did not reach the contract's body at all, which no test here
/// expects and which must therefore be loud.
fn unwrap_call(
    result: Result<
        Result<(), soroban_sdk::ConversionError>,
        Result<Error, soroban_sdk::InvokeError>,
    >,
) -> Result<(), Error> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => panic!("the return value did not decode: {e:?}"),
        Err(Ok(e)) => Err(e),
        Err(Err(e)) => panic!("the host refused the call: {e:?}"),
    }
}

/// One test's world: a deployed adapter, the two roles it was created with, and the mode it
/// is driven in.
pub struct Fixture {
    pub env: Env,
    pub mode: Mode,
    pub adapter: Address,
    pub governance: Address,
    pub keeper: Address,
    /// The scale the adapter was created with; 18 unless a test asked for another. The tests
    /// read it back through `decimals()` rather than from here, so that what they assert is
    /// what the contract stored.
    #[allow(dead_code)]
    pub decimals: u32,
}

impl Fixture {
    /// A fixture at 18 decimals, the scale every source of Rhumb is deployed at.
    pub fn new(mode: Mode) -> Option<Fixture> {
        Fixture::with_decimals(mode, 18)
    }

    /// A fixture at `decimals`. Panics when the constructor refuses, which is what the
    /// constructor tests assert.
    pub fn with_decimals(mode: Mode, decimals: u32) -> Option<Fixture> {
        Fixture::build(mode, decimals, Address::generate)
    }

    /// A fixture at 18 decimals whose keeper is the address `keeper` makes in the fixture's
    /// environment, for a test that needs the keeper to be a particular kind of address.
    pub fn with_keeper(mode: Mode, keeper: fn(&Env) -> Address) -> Option<Fixture> {
        Fixture::build(mode, 18, keeper)
    }

    fn build(mode: Mode, decimals: u32, keeper: fn(&Env) -> Address) -> Option<Fixture> {
        let wasm = adapter_wasm_for_tests()?;
        let env = test_env();
        env.mock_all_auths();
        env.ledger().with_mut(|l| {
            l.timestamp = JAN_1_2026;
            l.sequence_number = 1_000;
            l.min_persistent_entry_ttl = 4_096;
            l.max_entry_ttl = 6_312_000;
        });
        let governance = Address::generate(&env);
        let keeper = keeper(&env);

        // Both modes deploy the Wasm, so that the instance exists and its constructor ran the
        // way it runs on a chain. `Native` then calls the same entry points inside
        // `as_contract` on that instance, reading and writing the storage the deployment
        // created.
        let adapter = env.register(
            wasm.as_slice(),
            (governance.clone(), keeper.clone(), decimals),
        );

        // A freshly registered instance carries the test environment's minimum TTL, which is
        // far below the threshold of E-007 §3.1. On a chain a deployment is followed by the
        // keeper's first extend-footprint-TTL operation (spec §3.4), so the fixture puts the
        // entry where a kept adapter's sits; otherwise every test would start below the
        // threshold and the TTL tests could not tell the two sides of it apart.
        env.as_contract(&adapter, || {
            env.storage()
                .instance()
                .extend_ttl(crate::INSTANCE_TTL_THRESHOLD, crate::INSTANCE_TTL);
        });

        Some(Fixture {
            env,
            mode,
            adapter,
            governance,
            keeper,
            decimals,
        })
    }

    /// The client the `Wasm` mode drives, and that the authorisation tests use directly.
    pub fn client(&self) -> IssuerClient<'_> {
        IssuerClient::new(&self.env, &self.adapter)
    }

    /// `post` in the fixture's mode.
    pub fn post(&self, price: i128, timestamp: u64) -> Result<(), Error> {
        match self.mode {
            Mode::Wasm => unwrap_call(self.client().try_post(&price, &timestamp)),
            Mode::Native => self.env.as_contract(&self.adapter, || {
                IssuerAdapter::post(self.env.clone(), price, timestamp)
            }),
        }
    }

    /// `observe` in the fixture's mode.
    pub fn observe(&self) -> Option<(i128, u32, u64)> {
        match self.mode {
            Mode::Wasm => self.client().observe(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || IssuerAdapter::observe(self.env.clone())),
        }
    }

    /// `set_keeper` in the fixture's mode.
    pub fn set_keeper(&self, keeper: &Address) -> Result<(), Error> {
        match self.mode {
            Mode::Wasm => unwrap_call(self.client().try_set_keeper(keeper)),
            Mode::Native => self.env.as_contract(&self.adapter, || {
                IssuerAdapter::set_keeper(self.env.clone(), keeper.clone())
            }),
        }
    }

    /// `set_governance` in the fixture's mode.
    pub fn set_governance(&self, governance: &Address) -> Result<(), Error> {
        match self.mode {
            Mode::Wasm => unwrap_call(self.client().try_set_governance(governance)),
            Mode::Native => self.env.as_contract(&self.adapter, || {
                IssuerAdapter::set_governance(self.env.clone(), governance.clone())
            }),
        }
    }

    /// `keeper()` in the fixture's mode.
    pub fn keeper_view(&self) -> Option<Address> {
        match self.mode {
            Mode::Wasm => self.client().keeper(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || IssuerAdapter::keeper(self.env.clone())),
        }
    }

    /// `governance()` in the fixture's mode.
    pub fn governance_view(&self) -> Option<Address> {
        match self.mode {
            Mode::Wasm => self.client().governance(),
            Mode::Native => self.env.as_contract(&self.adapter, || {
                IssuerAdapter::governance(self.env.clone())
            }),
        }
    }

    /// `decimals()` in the fixture's mode.
    pub fn decimals_view(&self) -> Option<u32> {
        match self.mode {
            Mode::Wasm => self.client().decimals(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || IssuerAdapter::decimals(self.env.clone())),
        }
    }

    /// `latest()` in the fixture's mode.
    pub fn latest(&self) -> Option<Posting> {
        match self.mode {
            Mode::Wasm => self.client().latest(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || IssuerAdapter::latest(self.env.clone())),
        }
    }

    /// `version()` in the fixture's mode.
    pub fn version(&self) -> u32 {
        match self.mode {
            Mode::Wasm => self.client().version(),
            Mode::Native => self
                .env
                .as_contract(&self.adapter, || IssuerAdapter::version(self.env.clone())),
        }
    }

    /// The ledger's current timestamp.
    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    /// Moves the ledger `ledgers` forward, five seconds to the ledger, which is how an entry's
    /// TTL is spent.
    pub fn advance(&self, ledgers: u32) {
        self.env.ledger().with_mut(|l| {
            l.sequence_number += ledgers;
            l.timestamp += u64::from(ledgers) * 5;
        });
    }

    /// Asserts that everything the adapter has emitted so far is exactly `expected`, in
    /// order. The whole list is compared rather than the last entry, so that a test which
    /// expects no event proves that none was added and one which expects an event proves that
    /// no second one came with it.
    pub fn assert_events(&self, expected: &[&dyn Event]) {
        let mut want = soroban_sdk::Vec::new(&self.env);
        for e in expected {
            want.push_back((self.adapter.clone(), e.topics(&self.env), e.data(&self.env)));
        }
        assert_eq!(
            self.env.events().all().filter_by_contract(&self.adapter),
            want
        );
    }

    /// The adapter instance's remaining TTL, in ledgers.
    pub fn instance_ttl(&self) -> u32 {
        self.env.deployer().get_contract_instance_ttl(&self.adapter)
    }
}
